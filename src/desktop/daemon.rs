use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetForegroundWindow, GetMessageW,
    GetWindowThreadProcessId, PostQuitMessage, RegisterClassExW, TranslateMessage, MSG,
    SW_SHOWNORMAL, WNDCLASSEXW, WM_HOTKEY, WM_USER,
};

use crate::config::{self, DesktopBinding, DesktopConfig};
use crate::desktop::{awake, layout, remap, theme};
use crate::output::{CliError, CliResult};

const WM_AWAKE_EXPIRE: u32 = WM_USER + 4;
const WM_REMAP_HEALTH: u32 = WM_USER + 5;

pub(crate) struct DaemonOptions {
    pub(crate) quiet: bool,
    pub(crate) no_tray: bool,
}

pub(crate) fn run_daemon(opts: DaemonOptions) -> CliResult {
    let cfg = config::load_config();
    let desktop = cfg.desktop.clone();

    let quiet = opts.quiet || desktop.daemon.quiet;
    let no_tray = opts.no_tray || desktop.daemon.no_tray;
    let _ = no_tray;

    if !quiet {
        ui_println!("desktop daemon starting...");
        ui_println!("config: {}", config::config_path().display());
    }

    let hwnd = create_hidden_window().map_err(|e| {
        CliError::with_details(2, "Failed to create daemon window.", &[e.message.as_str()])
    })?;

    let mut registered: Vec<RegisteredHotkey> = Vec::new();
    for (idx, binding) in desktop.bindings.iter().enumerate() {
        let Some(parsed) = crate::desktop::hotkey::parse_hotkey(&binding.hotkey) else {
            if !quiet {
                ui_println!("hotkey ignored (invalid): {}", binding.hotkey);
            }
            continue;
        };
        let id = idx as i32 + 1;
        let ok = unsafe { RegisterHotKey(hwnd, id, parsed.modifiers, parsed.vk) };
        if ok == 0 {
            if !quiet {
                ui_println!("hotkey register failed: {}", binding.hotkey);
            }
            continue;
        }
        registered.push(RegisteredHotkey {
            id,
            binding: binding.clone(),
        });
    }

    if !quiet {
        ui_println!("hotkeys registered: {}", registered.len());
    }

    let probe_flag = Arc::new(AtomicBool::new(false));
    if !desktop.remaps.is_empty() || !desktop.snippets.is_empty() {
        remap::start_remap_thread(
            desktop.remaps.clone(),
            desktop.snippets.clone(),
            hwnd,
            Arc::clone(&probe_flag),
        );
        if !quiet {
            ui_println!("remap hook started: {} rules", desktop.remaps.len());
            ui_println!("snippets loaded: {}", desktop.snippets.len());
        }
    }

    let awake_state = awake::AwakeState::new();

    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if ret == 0 || ret == -1 {
                break;
            }

            match msg.message {
                WM_HOTKEY => {
                    let id = msg.wParam as i32;
                    if let Some(binding) = registered.iter().find(|r| r.id == id) {
                        handle_binding_action(
                            &binding.binding,
                            &desktop,
                            &awake_state,
                            quiet,
                        );
                    }
                }
                WM_AWAKE_EXPIRE => {
                    awake::cancel_awake(&awake_state);
                    if !quiet {
                        ui_println!("awake expired, state reset.");
                    }
                }
                WM_REMAP_HEALTH => {
                    ui_println!("remap hook lost, please restart daemon.");
                }
                _ => {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    for entry in &registered {
        unsafe { UnregisterHotKey(hwnd, entry.id) };
    }
    if !quiet {
        ui_println!("desktop daemon stopped.");
    }
    Ok(())
}

struct RegisteredHotkey {
    id: i32,
    binding: DesktopBinding,
}

fn handle_binding_action(
    binding: &DesktopBinding,
    desktop: &DesktopConfig,
    awake_state: &Arc<Mutex<awake::AwakeState>>,
    quiet: bool,
) {
    if let Some(app) = binding.app.as_deref() {
        let cur = foreground_process_name();
        if cur.is_empty() || !cur.eq_ignore_ascii_case(app) {
            return;
        }
    }

    let action = binding.action.trim();
    if action.is_empty() {
        return;
    }

    if let Some(cmd) = action.strip_prefix("run:") {
        if let Err(err) = spawn_run(cmd) {
            ui_println!("run failed: {}", err.message);
        }
        return;
    }
    if let Some(cmd) = action.strip_prefix("shell:") {
        if let Err(err) = spawn_shell(cmd) {
            ui_println!("shell failed: {}", err.message);
        }
        return;
    }
    if let Some(uri) = action.strip_prefix("uri:") {
        if let Err(err) = open_uri(uri) {
            ui_println!("uri open failed: {}", err.message);
        }
        return;
    }
    if let Some(name) = action.strip_prefix("layout_apply:") {
        if let Err(err) = apply_layout(name, desktop) {
            ui_println!("layout apply failed: {}", err.message);
        } else if !quiet {
            ui_println!("layout applied: {}", name.trim());
        }
        return;
    }

    match action {
        "theme_toggle" => {
            if let Ok(mode) = theme::toggle_theme() {
                if !quiet {
                    ui_println!("theme toggled: {}", mode.label());
                }
            }
        }
        "theme_light" => {
            if let Err(err) = theme::set_theme(&theme::ThemeMode::Light) {
                ui_println!("theme set failed: {}", err.message);
            }
        }
        "theme_dark" => {
            if let Err(err) = theme::set_theme(&theme::ThemeMode::Dark) {
                ui_println!("theme set failed: {}", err.message);
            }
        }
        "awake_toggle" => {
            let state = awake_state.lock().unwrap();
            let is_off = matches!(state.mode, awake::AwakeMode::Off);
            drop(state);
            let result = if is_off {
                awake::awake_indefinite(false, awake_state)
            } else {
                awake::cancel_awake(awake_state);
                Ok(())
            };
            if let Err(err) = result {
                ui_println!("awake toggle failed: {}", err.message);
            } else if !quiet {
                ui_println!("awake toggled");
            }
        }
        "daemon_stop" => unsafe {
            PostQuitMessage(0);
        },
        _ => {
            if !quiet {
                ui_println!("unsupported action: {}", action);
            }
        }
    }
}

fn spawn_run(raw: &str) -> Result<(), CliError> {
    let cmdline = raw.trim();
    if cmdline.is_empty() {
        return Err(CliError::new(2, "run command is empty."));
    }
    let parts: Vec<String> = cmdline
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if parts.is_empty() {
        return Err(CliError::new(2, "run command is empty."));
    }
    let exe = parts[0].clone();
    let args = parts.into_iter().skip(1).collect::<Vec<_>>();
    std::thread::spawn(move || {
        let _ = std::process::Command::new(exe).args(args).spawn();
    });
    Ok(())
}

fn spawn_shell(raw: &str) -> Result<(), CliError> {
    let cmdline = raw.trim();
    if cmdline.is_empty() {
        return Err(CliError::new(2, "shell command is empty."));
    }
    let cmd = cmdline.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("cmd").args(["/C", &cmd]).spawn();
    });
    Ok(())
}

fn open_uri(raw: &str) -> Result<(), CliError> {
    let uri = raw.trim();
    if uri.is_empty() {
        return Err(CliError::new(2, "uri is empty."));
    }
    let verb = to_wide("open");
    let target = to_wide(uri);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL as i32,
        )
    };
    if result as isize <= 32 {
        return Err(CliError::new(2, "ShellExecuteW failed."));
    }
    Ok(())
}

fn apply_layout(name: &str, desktop: &DesktopConfig) -> Result<(), CliError> {
    let name = name.trim();
    let layout_cfg = desktop
        .layouts
        .iter()
        .find(|l| l.name == name)
        .ok_or_else(|| CliError::new(2, format!("Layout not found: {name}")))?;

    let layout_type = layout_cfg.template.layout_type.to_lowercase();
    if layout_type != "grid" {
        return Err(CliError::with_details(
            2,
            format!("Unsupported layout type: {layout_type}"),
            &["Fix: use grid in Phase 1."],
        ));
    }

    let grid = layout::GridLayout {
        rows: layout_cfg.template.rows.unwrap_or(2).max(1) as usize,
        cols: layout_cfg.template.cols.unwrap_or(2).max(1) as usize,
        gap: layout_cfg.template.gap.unwrap_or(8) as i32,
        padding: 0,
        monitor: 0,
    };
    let template = layout::LayoutTemplate::Grid(grid);
    let monitor = layout::get_monitor_area(0)
        .ok_or_else(|| CliError::new(2, "Monitor 0 not found."))?;
    let zones = layout::compute_zones(&template, &monitor);
    if zones.is_empty() {
        return Err(CliError::new(2, "No zones computed."));
    }

    let mut assignments: Vec<(u32, layout::ZoneRect)> = Vec::new();
    for (app, zone_index) in &layout_cfg.bindings {
        if *zone_index >= zones.len() {
            return Err(CliError::new(2, format!("Zone index out of range: {zone_index}")));
        }
        if let Some(proc) = crate::proc::find_by_name(app)
            .into_iter()
            .find(|p| !p.window_title.trim().is_empty())
        {
            assignments.push((proc.pid, zones[*zone_index].clone()));
        }
    }

    if assignments.is_empty() {
        return Err(CliError::new(2, "No windows matched for layout."));
    }

    for (pid, zone) in assignments {
        let hwnd = crate::windows::window_api::find_hwnd_by_pid(pid)
            .map_err(|_| CliError::new(2, "Window handle not found."))?;
        let rect = crate::windows::window_api::WindowRect {
            left: zone.x,
            top: zone.y,
            right: zone.x + zone.w,
            bottom: zone.y + zone.h,
        };
        crate::windows::window_api::apply_window_rect(hwnd, rect)
            .map_err(|_| CliError::new(2, "Failed to apply layout."))?;
    }
    Ok(())
}

fn create_hidden_window() -> Result<HWND, CliError> {
    let class_name = to_wide("xun_desktop_daemon");
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };

    let mut wc: WNDCLASSEXW = unsafe { std::mem::zeroed() };
    wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(DefWindowProcW);
    wc.lpszClassName = class_name.as_ptr();
    wc.hInstance = instance;

    unsafe {
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err(CliError::new(2, "CreateWindowExW returned null."));
        }
        Ok(hwnd)
    }
}

fn foreground_process_name() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return String::new();
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return String::new();
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buf = vec![0u16; 32768];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return String::new();
        }
        let full = String::from_utf16_lossy(&buf[..size as usize]);
        Path::new(&full)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or(full)
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
