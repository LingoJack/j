use image::DynamicImage;

/// 加载图片（支持 URL 和本地路径）
pub fn load_image(source: &str) -> Result<DynamicImage, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let bytes = reqwest::blocking::get(source)
            .map_err(|e| e.to_string())?
            .bytes()
            .map_err(|e| e.to_string())?;
        image::load_from_memory(&bytes).map_err(|e| e.to_string())
    } else {
        let path = super::super::tools::expand_tilde(source);
        image::open(&path).map_err(|e| e.to_string())
    }
}
