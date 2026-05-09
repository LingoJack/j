use core_graphics::event::CGEventFlags;

use super::error::AicError;

/// macOS virtual key code type
pub type KeyCode = u16;

/// Resolve a human-readable key name to a macOS virtual key code.
pub fn resolve_key(name: &str) -> Result<KeyCode, AicError> {
    let code = match name.to_lowercase().as_str() {
        // Letters
        "a" => 0x00,
        "s" => 0x01,
        "d" => 0x02,
        "f" => 0x03,
        "h" => 0x04,
        "g" => 0x05,
        "z" => 0x06,
        "x" => 0x07,
        "c" => 0x08,
        "v" => 0x09,
        "b" => 0x0B,
        "q" => 0x0C,
        "w" => 0x0D,
        "e" => 0x0E,
        "r" => 0x0F,
        "y" => 0x10,
        "t" => 0x11,
        "1" | "!" => 0x12,
        "2" | "@" => 0x13,
        "3" | "#" => 0x14,
        "4" | "$" => 0x15,
        "6" | "^" => 0x16,
        "5" | "%" => 0x17,
        "=" | "+" => 0x18,
        "9" | "(" => 0x19,
        "7" | "&" => 0x1A,
        "-" | "_" => 0x1B,
        "8" | "*" => 0x1C,
        "0" | ")" => 0x1D,
        "]" | "}" => 0x1E,
        "o" => 0x1F,
        "u" => 0x20,
        "[" | "{" => 0x21,
        "i" => 0x22,
        "p" => 0x23,
        "l" => 0x25,
        "j" => 0x26,
        "'" | "\"" | "quote" => 0x27,
        "k" => 0x28,
        ";" | ":" | "semicolon" => 0x29,
        "\\" | "|" | "backslash" => 0x2A,
        "," | "<" | "comma" => 0x2B,
        "/" | "?" | "slash" => 0x2C,
        "n" => 0x2D,
        "m" => 0x2E,
        "." | ">" | "period" | "dot" => 0x2F,
        "`" | "~" | "backtick" | "grave" => 0x32,

        // Special keys
        "return" | "enter" => 0x24,
        "tab" => 0x30,
        "space" | " " => 0x31,
        "delete" | "backspace" => 0x33,
        "escape" | "esc" => 0x35,
        "capslock" => 0x39,

        // Arrow keys
        "left" => 0x7B,
        "right" => 0x7C,
        "down" => 0x7D,
        "up" => 0x7E,

        // Function keys
        "f1" => 0x7A,
        "f2" => 0x78,
        "f3" => 0x63,
        "f4" => 0x76,
        "f5" => 0x60,
        "f6" => 0x61,
        "f7" => 0x62,
        "f8" => 0x64,
        "f9" => 0x65,
        "f10" => 0x6D,
        "f11" => 0x67,
        "f12" => 0x6F,

        // Navigation
        "home" => 0x73,
        "end" => 0x77,
        "pageup" => 0x74,
        "pagedown" => 0x79,
        "forwarddelete" | "fwddelete" => 0x75,

        // Modifier keys (as standalone keys)
        "cmd" | "command" => 0x37,
        "shift" => 0x38,
        "alt" | "option" => 0x3A,
        "ctrl" | "control" => 0x3B,
        "rightshift" => 0x3C,
        "rightoption" | "rightalt" => 0x3D,
        "rightcontrol" | "rightctrl" => 0x3E,
        "fn" | "function" => 0x3F,

        _ => return Err(AicError::UnknownKey(name.to_string())),
    };
    Ok(code)
}

/// Resolve a modifier name to CGEventFlags.
pub fn resolve_modifier(name: &str) -> Result<CGEventFlags, AicError> {
    match name.to_lowercase().as_str() {
        "cmd" | "command" => Ok(CGEventFlags::CGEventFlagCommand),
        "shift" => Ok(CGEventFlags::CGEventFlagShift),
        "alt" | "option" => Ok(CGEventFlags::CGEventFlagAlternate),
        "ctrl" | "control" => Ok(CGEventFlags::CGEventFlagControl),
        _ => Err(AicError::UnknownModifier(name.to_string())),
    }
}

/// Check if a key name is a modifier.
pub fn is_modifier(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "cmd" | "command" | "shift" | "alt" | "option" | "ctrl" | "control"
    )
}
