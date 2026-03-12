use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VK_BACK, VK_CONTROL, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_RETURN,
    VK_SPACE, VK_TAB, VK_V,
};

use crate::config::DesktopSnippet;

pub(crate) const REMAP_TAG: usize = 0xA1B2_C3D4;

const ELECTRON_APPS: &[&str] = &[
    "code.exe",
    "cursor.exe",
    "discord.exe",
    "figma.exe",
    "notion.exe",
    "obsidian.exe",
    "slack.exe",
    "teams.exe",
];

const UWP_HOSTS: &[&str] = &[
    "applicationframehost.exe",
    "winstore.app.exe",
];

pub(crate) type CharBuf = Arc<Mutex<VecDeque<char>>>;

pub(crate) fn new_char_buf() -> CharBuf {
    Arc::new(Mutex::new(VecDeque::with_capacity(64)))
}

pub(crate) fn on_key(
    vk: u32,
    snippets: &[DesktopSnippet],
    cur_app: &str,
    buf: &CharBuf,
    tag: usize,
) -> bool {
    if vk == VK_BACK as u32 {
        buf.lock().unwrap().pop_back();
        return false;
    }

    let is_terminator = matches!(
        vk as u16,
        v if v == VK_SPACE || v == VK_RETURN || v == VK_TAB
    ) || is_punctuation(vk as u8);

    if let Some(c) = vk_to_char(vk) {
        let mut guard = buf.lock().unwrap();
        if guard.len() >= 64 {
            guard.pop_front();
        }
        guard.push_back(c);
    }

    {
        let guard = buf.lock().unwrap();
        let s: String = guard.iter().collect();
        for snippet in snippets {
            if !snippet.immediate {
                continue;
            }
            if !app_matches(&snippet.app, cur_app) {
                continue;
            }
            if s.ends_with(&snippet.trigger) {
                drop(guard);
                trigger_snippet(snippet, None, buf, tag, cur_app);
                return true;
            }
        }
    }

    if is_terminator {
        let guard = buf.lock().unwrap();
        let s: String = guard.iter().rev().skip(1).collect::<String>()
            .chars().rev().collect();
        for snippet in snippets {
            if snippet.immediate {
                continue;
            }
            if !app_matches(&snippet.app, cur_app) {
                continue;
            }
            if s.ends_with(&snippet.trigger) {
                drop(guard);
                trigger_snippet(snippet, Some(vk as u16), buf, tag, cur_app);
                return true;
            }
        }
    }

    false
}

fn app_matches(app: &Option<String>, cur_app: &str) -> bool {
    match app {
        None => true,
        Some(a) => a.eq_ignore_ascii_case(cur_app),
    }
}

fn trigger_snippet(
    snippet: &DesktopSnippet,
    terminator_vk: Option<u16>,
    buf: &CharBuf,
    tag: usize,
    cur_app: &str,
) {
    let expanded = expand_dynamic(&snippet.expand);
    let delete_count = snippet.trigger.len() + if terminator_vk.is_some() { 1 } else { 0 };

    send_backspaces(delete_count, tag);

    let prefer_clipboard = snippet
        .paste
        .as_deref()
        .map(|v| v.eq_ignore_ascii_case("clipboard"))
        .unwrap_or(false);

    if prefer_clipboard || is_clipboard_app(cur_app) || expanded.chars().count() > 50 {
        inject_via_clipboard(&expanded, tag);
    } else {
        send_unicode_string(&expanded, tag);
    }

    if let Some(vk) = terminator_vk {
        if !snippet.immediate {
            send_single_key(vk, tag);
        }
    }

    buf.lock().unwrap().clear();
}

fn expand_dynamic(template: &str) -> String {
    let now = chrono::Local::now();
    template
        .replace("{date}", &now.format("%Y-%m-%d").to_string())
        .replace("{time}", &now.format("%H:%M:%S").to_string())
        .replace("{datetime}", &now.format("%Y-%m-%d %H:%M:%S").to_string())
        .replace("{uuid}", &uuid::Uuid::new_v4().to_string())
}

fn is_clipboard_app(exe: &str) -> bool {
    let lower = exe.to_lowercase();
    ELECTRON_APPS.iter().any(|&a| lower == a)
        || UWP_HOSTS.iter().any(|&a| lower == a)
}

pub(crate) fn send_unicode_string(s: &str, tag: usize) {
    if s.is_empty() {
        return;
    }
    unsafe {
        let inputs: Vec<INPUT> = s.encode_utf16()
            .flat_map(|c| {
                [
                    make_keyboard_input(0, c, KEYEVENTF_UNICODE, tag),
                    make_keyboard_input(0, c, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP, tag),
                ]
            })
            .collect();
        if inputs.is_empty() {
            return;
        }
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

pub(crate) fn send_single_key(vk: u16, tag: usize) {
    unsafe {
        let inputs = [
            make_keyboard_input(vk, 0, 0, tag),
            make_keyboard_input(vk, 0, KEYEVENTF_KEYUP, tag),
        ];
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

fn send_backspaces(n: usize, tag: usize) {
    for _ in 0..n {
        send_single_key(VK_BACK, tag);
    }
}

fn inject_via_clipboard(text: &str, tag: usize) {
    let old = get_clipboard_text();
    set_clipboard_text(text);

    unsafe {
        let inputs = [
            make_keyboard_input(VK_CONTROL, 0, 0, tag),
            make_keyboard_input(VK_V, 0, 0, tag),
            make_keyboard_input(VK_V, 0, KEYEVENTF_KEYUP, tag),
            make_keyboard_input(VK_CONTROL, 0, KEYEVENTF_KEYUP, tag),
        ];
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }

    let old_clone = old.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if let Some(s) = old_clone {
            set_clipboard_text(&s);
        }
    });
}

fn make_keyboard_input(vk: u16, scan: u16, flags: u32, tag: usize) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: tag,
            },
        },
    }
}

fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let handle = GetClipboardData(13u32);
        if handle.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        let _ = GlobalUnlock(handle);
        let _ = CloseClipboard();
        Some(text)
    }
}

fn set_clipboard_text(text: &str) {
    unsafe {
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let size = wide.len() * 2;
        let hmem = GlobalAlloc(GMEM_MOVEABLE, size);
        if hmem.is_null() {
            return;
        }
        let ptr = GlobalLock(hmem) as *mut u16;
        if ptr.is_null() {
            return;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        let _ = GlobalUnlock(hmem);
        if OpenClipboard(std::ptr::null_mut()) != 0 {
            let _ = EmptyClipboard();
            let _ = SetClipboardData(13u32, hmem);
            let _ = CloseClipboard();
        }
    }
}

fn vk_to_char(vk: u32) -> Option<char> {
    if (0x41..=0x5A).contains(&vk) {
        return Some((vk as u8 + 32) as char);
    }
    if (0x30..=0x39).contains(&vk) {
        return Some(char::from(vk as u8));
    }
    match vk as u16 {
        v if v == VK_SPACE => Some(' '),
        v if v == VK_OEM_PERIOD => Some('.'),
        v if v == VK_OEM_COMMA => Some(','),
        v if v == VK_OEM_MINUS => Some('-'),
        v if v == VK_OEM_PLUS => Some('+'),
        _ => None,
    }
}

fn is_punctuation(vk: u8) -> bool {
    matches!(vk, 0xBE | 0xBC | 0xBF | 0xBA)
}
