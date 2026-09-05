//! 剪贴板图片粘贴支持
//!
//! 终端的 bracketed paste 只能传输文本；图片不会随粘贴事件进入 TUI。
//! 因此监听 Ctrl+V 按键后，直接读取系统剪贴板：
//! - 有图片 → 转为 PNG base64 附件，随下一条用户消息以多模态 ContentPart 发送
//! - 无图片有文本 → 回退为普通文本粘贴（与 Event::Paste 行为一致）
//! - 都没有 → toast 提示

use arboard::Clipboard;
use image::{ImageBuffer, ImageFormat, RgbaImage};
use std::io::Cursor;

use crate::command::chat::app::chat_app::ChatApp;
use crate::command::chat::app::ui_state::PendingImage;
use crate::command::chat::storage::ImageData;

use base64::{engine::general_purpose, Engine as _};

/// 剪贴板内容类型
enum ClipboardContent {
    /// 剪贴板中的图片（PNG base64 编码后）
    Image(PendingImage),
    /// 剪贴板中的文本
    Text(String),
    /// 剪贴板为空或不支持
    Empty,
}

/// 从系统剪贴板读取图片（RGBA → PNG → base64）
fn read_clipboard_image() -> Result<PendingImage, String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
    let img = clipboard
        .get_image()
        .map_err(|e| format!("剪贴板中没有图片: {e}"))?;

    let (width, height) = (img.width as u32, img.height as u32);
    if width == 0 || height == 0 {
        return Err("剪贴板图片尺寸无效".to_string());
    }
    if img.bytes.len() != (width as usize) * (height as usize) * 4 {
        return Err("剪贴板图片数据不完整".to_string());
    }

    // RGBA8 → PNG
    let rgba: RgbaImage = ImageBuffer::from_raw(width, height, img.bytes.to_vec())
        .ok_or_else(|| "剪贴板图片数据无效".to_string())?;
    let mut png_bytes = Vec::new();
    rgba.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|e| format!("图片编码失败: {e}"))?;

    Ok(PendingImage {
        data: ImageData {
            base64: general_purpose::STANDARD.encode(&png_bytes),
            media_type: "image/png".to_string(),
        },
        width,
        height,
        size_bytes: png_bytes.len(),
    })
}

/// 读取剪贴板内容（优先图片，其次文本）
fn read_clipboard_content() -> ClipboardContent {
    if let Ok(img) = read_clipboard_image() {
        return ClipboardContent::Image(img);
    }
    match Clipboard::new().and_then(|mut c| c.get_text()) {
        Ok(text) if !text.trim().is_empty() => ClipboardContent::Text(text),
        _ => ClipboardContent::Empty,
    }
}

impl ChatApp {
    /// Ctrl+V：从剪贴板粘贴图片（或回退为文本）
    pub fn paste_from_clipboard(&mut self) {
        match read_clipboard_content() {
            ClipboardContent::Image(img) => {
                let supports_vision = self
                    .active_provider()
                    .map(|p| p.supports_vision)
                    .unwrap_or(false);
                self.ui.pending_images.push(img);
                let count = self.ui.pending_images.len();
                if supports_vision {
                    self.show_toast(
                        format!("已添加剪贴板图片（共 {count} 张，Enter 发送，Backspace 移除）"),
                        false,
                    );
                } else {
                    self.show_toast(
                        format!("已添加剪贴板图片（共 {count} 张）注意：当前模型未开启 supports_vision，图片可能无法被识别"),
                        true,
                    );
                }
            }
            ClipboardContent::Text(text) => {
                // 回退：文本粘贴（与 Event::Paste / bracketed paste 行为一致）
                for c in text.chars() {
                    if c == '\r' {
                        continue; // 忽略 \r，统一用 \n 换行
                    }
                    if c == '\n' {
                        self.ui.input_buffer.insert_newline();
                    } else {
                        self.ui.input_buffer.insert_char(c);
                    }
                }
            }
            ClipboardContent::Empty => {
                self.show_toast("剪贴板中没有图片或文本", true);
            }
        }
    }
}
