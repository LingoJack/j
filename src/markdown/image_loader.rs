use image::DynamicImage;

/// 加载图片（支持 http/https URL、file:// URI 和本地路径）
pub fn load_image(source: &str) -> Result<DynamicImage, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let bytes = reqwest::blocking::get(source)
            .map_err(|e| e.to_string())?
            .bytes()
            .map_err(|e| e.to_string())?;
        image::load_from_memory(&bytes).map_err(|e| e.to_string())
    } else {
        // 处理 file:// 协议：提取实际文件路径
        let path = if let Some(stripped) = source.strip_prefix("file://") {
            stripped.to_string()
        } else {
            crate::command::chat::tools::expand_tilde(source)
        };
        image::open(&path).map_err(|e| e.to_string())
    }
}
