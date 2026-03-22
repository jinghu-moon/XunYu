use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_BACK, VK_CAPITAL, VK_DELETE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9,
    VK_F10, VK_F11, VK_F12, VK_HOME, VK_INSERT, VK_LEFT, VK_NEXT, VK_NUMLOCK, VK_OEM_1, VK_OEM_2,
    VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD,
    VK_OEM_PLUS, VK_PAUSE, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SCROLL, VK_SNAPSHOT, VK_SPACE, VK_TAB,
    VK_UP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParsedHotkey {
    pub(crate) modifiers: HOT_KEY_MODIFIERS,
    pub(crate) vk: u32,
}

pub(crate) const UNREMAPPABLE_KEYS: &[(&str, &str)] = &[
    ("ctrl+alt+del", "内核级安全序列，无法拦截"),
    ("win+l", "系统锁屏，hook 层以下处理"),
    ("win+shift+s", "系统截图快捷键，无法禁用"),
    ("fn", "硬件层处理，不经过 OS"),
];

pub(crate) fn parse_hotkey(raw: &str) -> Option<ParsedHotkey> {
    let mut modifiers: HOT_KEY_MODIFIERS = 0;
    let mut vk: Option<u32> = None;

    for part in raw.to_lowercase().split('+') {
        match part.trim() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" | "windows" => modifiers |= MOD_WIN,
            key => vk = str_to_vk(key),
        }
    }

    vk.map(|v| ParsedHotkey { modifiers, vk: v })
}

pub(crate) fn str_to_vk(s: &str) -> Option<u32> {
    let result = match s {
        "f1" => VK_F1,
        "f2" => VK_F2,
        "f3" => VK_F3,
        "f4" => VK_F4,
        "f5" => VK_F5,
        "f6" => VK_F6,
        "f7" => VK_F7,
        "f8" => VK_F8,
        "f9" => VK_F9,
        "f10" => VK_F10,
        "f11" => VK_F11,
        "f12" => VK_F12,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "home" => VK_HOME,
        "end" => VK_END,
        "pgup" | "pageup" => VK_PRIOR,
        "pgdn" | "pagedown" => VK_NEXT,
        "ins" | "insert" => VK_INSERT,
        "del" | "delete" => VK_DELETE,
        "space" => VK_SPACE,
        "enter" | "return" => VK_RETURN,
        "tab" => VK_TAB,
        "esc" | "escape" => VK_ESCAPE,
        "backspace" => VK_BACK,
        "capslock" => VK_CAPITAL,
        "pause" => VK_PAUSE,
        "printscreen" => VK_SNAPSHOT,
        "scrolllock" => VK_SCROLL,
        "numlock" => VK_NUMLOCK,
        ";" | "semicolon" => VK_OEM_1,
        "=" | "equals" => VK_OEM_PLUS,
        "," | "comma" => VK_OEM_COMMA,
        "-" | "minus" => VK_OEM_MINUS,
        "." | "period" => VK_OEM_PERIOD,
        "/" | "slash" => VK_OEM_2,
        "`" | "backtick" => VK_OEM_3,
        "[" | "leftbracket" => VK_OEM_4,
        "\\" | "backslash" => VK_OEM_5,
        "]" | "rightbracket" => VK_OEM_6,
        "'" | "quote" => VK_OEM_7,
        _ => {
            if s.len() == 1 {
                let c = s.chars().next()?;
                if c.is_ascii_alphabetic() {
                    return Some(c.to_ascii_uppercase() as u32);
                }
                if c.is_ascii_digit() {
                    return Some(c as u32);
                }
            }
            return None;
        }
    };
    Some(result as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hotkey_basic() {
        let hk = parse_hotkey("ctrl+alt+t").expect("parse");
        assert_eq!(hk.vk, 'T' as u32);
        assert!(hk.modifiers & MOD_CONTROL != 0);
        assert!(hk.modifiers & MOD_ALT != 0);
    }

    #[test]
    fn parse_hotkey_special_keys() {
        let hk = parse_hotkey("win+shift+f1").expect("parse");
        assert_eq!(hk.vk, VK_F1 as u32);
        assert!(hk.modifiers & MOD_WIN != 0);
        assert!(hk.modifiers & MOD_SHIFT != 0);
    }

    #[test]
    fn parse_hotkey_invalid() {
        assert!(parse_hotkey("ctrl+alt+").is_none());
        assert!(parse_hotkey("unknown").is_none());
    }
}
