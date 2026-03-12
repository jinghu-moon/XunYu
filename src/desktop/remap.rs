use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::{
    GetCurrentThread, OpenProcess, QueryFullProcessImageNameW, SetThreadPriority,
    PROCESS_QUERY_LIMITED_INFORMATION, THREAD_PRIORITY_HIGHEST,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, HOT_KEY_MODIFIERS, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_LWIN, VK_MENU,
    VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    PostMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN, WM_USER,
};

use crate::config::{DesktopRemap, DesktopSnippet};

use super::hotkey::{parse_hotkey, ParsedHotkey};
use super::snippet::{self, CharBuf};

const PROBE_TAG: usize = 0xB2C3_D4E5;
const PROBE_VK: u16 = 0xFF;

#[derive(Debug, Clone)]
pub(crate) enum RemapTarget {
    Hotkey { modifiers: HOT_KEY_MODIFIERS, vk: u16 },
    Disable,
    Text(String),
}

impl RemapTarget {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let value = raw.trim();
        if value.eq_ignore_ascii_case("disable") {
            return Some(Self::Disable);
        }
        if let Some(text) = value.strip_prefix("text:") {
            return Some(Self::Text(text.to_string()));
        }
        let hotkey = parse_hotkey(value)?;
        Some(Self::Hotkey {
            modifiers: hotkey.modifiers,
            vk: hotkey.vk as u16,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemapRule {
    from: ParsedHotkey,
    target: RemapTarget,
    app_lower: Option<String>,
    exact: bool,
}

pub(crate) fn parse_remap(remap: &DesktopRemap) -> Option<RemapRule> {
    let from = parse_hotkey(&remap.from)?;
    let target = RemapTarget::parse(&remap.to)?;
    let app_lower = remap.app.as_ref().map(|app| app.to_lowercase());
    Some(RemapRule {
        from,
        target,
        app_lower,
        exact: remap.exact,
    })
}

struct HookContext {
    app_rules: Vec<RemapRule>,
    global_rules: Vec<RemapRule>,
    snippets: Vec<DesktopSnippet>,
    char_buf: CharBuf,
    probe_received: Arc<AtomicBool>,
    has_app_filter: bool,
}

static mut HOOK_CTX: Option<*mut HookContext> = None;
static mut HOOK_HANDLE: HHOOK = std::ptr::null_mut();

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        // SAFETY: Forwarding to next hook preserves system chain semantics.
        return unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) };
    }

    // SAFETY: lparam points to a valid KBDLLHOOKSTRUCT for low-level hooks.
    let kbd = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
    if kbd.dwExtraInfo == snippet::REMAP_TAG {
        // SAFETY: Forwarding to next hook preserves system chain semantics.
        return unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) };
    }
    if kbd.dwExtraInfo == PROBE_TAG {
        if let Some(ctx_ptr) = unsafe { HOOK_CTX } {
            // SAFETY: HOOK_CTX points to a valid HookContext for the hook thread.
            unsafe { (*ctx_ptr).probe_received.store(true, Ordering::Relaxed) };
        }
        return 1;
    }

    let is_key_down = wparam as u32 == WM_KEYDOWN || wparam as u32 == WM_SYSKEYDOWN;
    if !is_key_down {
        // SAFETY: Forwarding to next hook preserves system chain semantics.
        return unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) };
    }

    let Some(ctx_ptr) = (unsafe { HOOK_CTX }) else {
        // SAFETY: Forwarding to next hook preserves system chain semantics.
        return unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) };
    };
    // SAFETY: HOOK_CTX points to a valid HookContext for the hook thread.
    let ctx = unsafe { &*ctx_ptr };

    let (cur_app, cur_app_lower) = if ctx.has_app_filter {
        let name = foreground_process_name();
        (name.clone(), name.to_lowercase())
    } else {
        (String::new(), String::new())
    };

    if !ctx.snippets.is_empty() {
        let handled = snippet::on_key(
            kbd.vkCode,
            &ctx.snippets,
            &cur_app,
            &ctx.char_buf,
            snippet::REMAP_TAG,
        );
        if handled {
            return 1;
        }
    }

    let modifiers = current_modifiers();
    if apply_rules(&ctx.app_rules, kbd, &cur_app_lower, modifiers) {
        return 1;
    }
    if apply_rules(&ctx.global_rules, kbd, &cur_app_lower, modifiers) {
        return 1;
    }

    // SAFETY: Forwarding to next hook preserves system chain semantics.
    unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) }
}

pub(crate) fn start_remap_thread(
    remaps: Vec<DesktopRemap>,
    snippets: Vec<DesktopSnippet>,
    main_hwnd: HWND,
    probe_flag: Arc<AtomicBool>,
) {
    let rules: Vec<RemapRule> = remaps.iter().filter_map(parse_remap).collect();
    let mut app_rules = Vec::new();
    let mut global_rules = Vec::new();
    for rule in rules {
        if rule.app_lower.is_some() {
            app_rules.push(rule);
        } else {
            global_rules.push(rule);
        }
    }
    let has_app_filter = !app_rules.is_empty() || snippets.iter().any(|s| s.app.is_some());
    let char_buf = snippet::new_char_buf();
    let probe_flag_clone = Arc::clone(&probe_flag);

    std::thread::Builder::new()
        .name("xun-desktop-remap-hook".to_string())
        .spawn(move || unsafe {
            // SAFETY: Thread priority change is best-effort and scoped to this thread.
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);

            let ctx = Box::new(HookContext {
                app_rules,
                global_rules,
                snippets,
                char_buf,
                probe_received: probe_flag_clone,
                has_app_filter,
            });
            HOOK_CTX = Some(Box::into_raw(ctx));

            HOOK_HANDLE = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                std::ptr::null_mut(),
                0,
            );
            if HOOK_HANDLE.is_null() {
                let ctx_ptr = HOOK_CTX;
                HOOK_CTX = None;
                if let Some(ptr) = ctx_ptr {
                    drop(Box::from_raw(ptr));
                }
                return;
            }

            let mut msg: MSG = std::mem::zeroed();
            loop {
                let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if ret == 0 || ret == -1 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(HOOK_HANDLE);
            HOOK_HANDLE = std::ptr::null_mut();
            let ctx_ptr = HOOK_CTX;
            HOOK_CTX = None;
            if let Some(ptr) = ctx_ptr {
                drop(Box::from_raw(ptr));
            }
        })
        .expect("Failed to spawn remap hook thread");

    start_health_check(probe_flag, main_hwnd as isize);
}

fn apply_rules(
    rules: &[RemapRule],
    kbd: &KBDLLHOOKSTRUCT,
    cur_app_lower: &str,
    modifiers: HOT_KEY_MODIFIERS,
) -> bool {
    for rule in rules {
        if !app_matches(rule, cur_app_lower) {
            continue;
        }
        if !match_hotkey(&rule.from, kbd.vkCode, modifiers) {
            continue;
        }
        match &rule.target {
            RemapTarget::Hotkey { modifiers, vk } => {
                send_hotkey(*modifiers, *vk, snippet::REMAP_TAG);
            }
            RemapTarget::Text(text) => {
                snippet::send_unicode_string(text, snippet::REMAP_TAG);
            }
            RemapTarget::Disable => {}
        }
        return true;
    }
    false
}

fn app_matches(rule: &RemapRule, cur_app_lower: &str) -> bool {
    let Some(app_lower) = &rule.app_lower else {
        return true;
    };
    if cur_app_lower.is_empty() {
        return false;
    }
    if rule.exact {
        cur_app_lower == app_lower
    } else {
        cur_app_lower.contains(app_lower)
    }
}

fn match_hotkey(from: &ParsedHotkey, vk: u32, modifiers: HOT_KEY_MODIFIERS) -> bool {
    vk == from.vk && modifiers == from.modifiers
}

fn current_modifiers() -> HOT_KEY_MODIFIERS {
    let mut mods: HOT_KEY_MODIFIERS = 0;
    if is_key_down(VK_CONTROL as i32) {
        mods |= MOD_CONTROL;
    }
    if is_key_down(VK_MENU as i32) {
        mods |= MOD_ALT;
    }
    if is_key_down(VK_SHIFT as i32) {
        mods |= MOD_SHIFT;
    }
    if is_key_down(VK_LWIN as i32) || is_key_down(VK_RWIN as i32) {
        mods |= MOD_WIN;
    }
    mods
}

fn is_key_down(vk: i32) -> bool {
    // SAFETY: GetAsyncKeyState is a pure Win32 query and does not mutate state.
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000u16) != 0 }
}

fn send_hotkey(modifiers: HOT_KEY_MODIFIERS, vk: u16, tag: usize) {
    let mut down_keys = Vec::new();
    if modifiers & MOD_CONTROL != 0 {
        down_keys.push(VK_CONTROL);
    }
    if modifiers & MOD_ALT != 0 {
        down_keys.push(VK_MENU);
    }
    if modifiers & MOD_SHIFT != 0 {
        down_keys.push(VK_SHIFT);
    }
    if modifiers & MOD_WIN != 0 {
        down_keys.push(VK_LWIN);
    }

    let mut inputs: Vec<INPUT> = Vec::with_capacity(down_keys.len() * 2 + 2);
    for key in &down_keys {
        inputs.push(make_keyboard_input(*key, 0, tag));
    }
    inputs.push(make_keyboard_input(vk, 0, tag));
    inputs.push(make_keyboard_input(vk, KEYEVENTF_KEYUP, tag));
    for key in down_keys.iter().rev() {
        inputs.push(make_keyboard_input(*key, KEYEVENTF_KEYUP, tag));
    }

    if inputs.is_empty() {
        return;
    }
    // SAFETY: INPUT array is fully initialized and lives for the call duration.
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

fn make_keyboard_input(vk: u16, flags: u32, tag: usize) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: tag,
            },
        },
    }
}

fn foreground_process_name() -> String {
    // SAFETY: Win32 APIs used here do not invalidate Rust references and handle errors locally.
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

fn start_health_check(probe_flag: Arc<AtomicBool>, main_hwnd: isize) {
    std::thread::Builder::new()
        .name("xun-desktop-remap-health".to_string())
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(30));

            probe_flag.store(false, Ordering::Relaxed);
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: PROBE_VK,
                        wScan: 0,
                        dwFlags: 0,
                        time: 0,
                        dwExtraInfo: PROBE_TAG,
                    },
                },
            };
            // SAFETY: INPUT is initialized and lives for the call duration.
            unsafe {
                SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
            if !probe_flag.load(Ordering::Relaxed) && main_hwnd != 0 {
                let hwnd = main_hwnd as HWND;
                // SAFETY: PostMessageW is safe with a valid HWND or null check.
                unsafe {
                    let _ = PostMessageW(hwnd, WM_USER + 5, 0, 0);
                }
            }
        })
        .expect("Failed to spawn remap health thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_remap_target_hotkey() {
        let target = RemapTarget::parse("alt+1").expect("target");
        match target {
            RemapTarget::Hotkey { modifiers, vk } => {
                assert!(modifiers & MOD_ALT != 0);
                assert_eq!(vk, '1' as u16);
            }
            _ => panic!("expected hotkey target"),
        }
    }

    #[test]
    fn parse_remap_target_disable() {
        let target = RemapTarget::parse("disable").expect("target");
        assert!(matches!(target, RemapTarget::Disable));
    }

    #[test]
    fn parse_remap_target_text() {
        let target = RemapTarget::parse("text:hello").expect("target");
        match target {
            RemapTarget::Text(value) => assert_eq!(value, "hello"),
            _ => panic!("expected text target"),
        }
    }
}
