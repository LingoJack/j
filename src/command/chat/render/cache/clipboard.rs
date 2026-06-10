//! 剪贴板操作

/// 复制内容到系统剪贴板（使用 arboard 库，支持 macOS/Linux/Windows）
pub fn copy_to_clipboard(content: &str) -> bool {
    use arboard::Clipboard;

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    clipboard.set_text(content).is_ok()
}
