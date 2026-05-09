use std::path::Path;

use core_graphics::display::CGDisplay;
use image::RgbaImage;

use super::error::AicError;

/// Capture the main display and return the image with the Retina scale factor.
/// Scale factor = pixel width / point width (1.0 on non-Retina, 2.0 on Retina).
pub fn capture_screen() -> Result<(RgbaImage, f64), AicError> {
    let display = CGDisplay::main();
    let bounds = display.bounds();

    let cg_image = CGDisplay::image(&display).ok_or_else(|| {
        AicError::ScreenshotFailed(
            "CGDisplayCreateImage returned null — check Screen Recording permission".into(),
        )
    })?;

    let width = cg_image.width();
    let height = cg_image.height();
    let bytes_per_row = cg_image.bytes_per_row();
    let data = cg_image.data();
    let raw_bytes = data.bytes();

    let scale = width as f64 / bounds.size.width;

    // Convert BGRA to RGBA
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let row_start = row * bytes_per_row;
        for col in 0..width {
            let offset = row_start + col * 4;
            let b = raw_bytes[offset];
            let g = raw_bytes[offset + 1];
            let r = raw_bytes[offset + 2];
            let a = raw_bytes[offset + 3];
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(a);
        }
    }

    let img = RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| AicError::ImageEncodingFailed("failed to create image buffer".into()))?;

    Ok((img, scale))
}

/// Output an RgbaImage to file or as base64 string.
/// Returns the base64 string when `as_base64` is true and `output` is None.
pub fn output_image_to_base64(img: &RgbaImage) -> Result<String, AicError> {
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| AicError::ImageEncodingFailed(e.to_string()))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

/// Save an RgbaImage to a file.
pub fn save_image(img: &RgbaImage, path: &str) -> Result<(), AicError> {
    img.save(Path::new(path))
        .map_err(|e| AicError::ImageEncodingFailed(e.to_string()))
}
