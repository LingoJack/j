use image::{Rgba, RgbaImage};

// --- drawing primitives ---

pub(crate) fn blend_pixel(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let bg = *img.get_pixel(x as u32, y as u32);
    let a = color.0[3] as f64 / 255.0;
    let inv = 1.0 - a;
    let r = (color.0[0] as f64 * a + bg.0[0] as f64 * inv) as u8;
    let g = (color.0[1] as f64 * a + bg.0[1] as f64 * inv) as u8;
    let b = (color.0[2] as f64 * a + bg.0[2] as f64 * inv) as u8;
    img.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
}

pub(crate) fn draw_line(
    img: &mut RgbaImage,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    thickness: f64,
    color: Rgba<u8>,
) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return;
    }
    let steps = (len * 2.0) as usize;
    let half_t = thickness / 2.0;
    // Normal vector (perpendicular)
    let nx = -dy / len;
    let ny = dx / len;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let cx = x1 + dx * t;
        let cy = y1 + dy * t;
        let spread = half_t.ceil() as i32;
        for j in -spread..=spread {
            let fj = j as f64;
            if fj.abs() > half_t {
                continue;
            }
            let px = (cx + nx * fj) as i32;
            let py = (cy + ny * fj) as i32;
            blend_pixel(img, px, py, color);
        }
    }
}

pub(crate) fn draw_rect(
    img: &mut RgbaImage,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    thickness: f64,
    color: Rgba<u8>,
) {
    // Top
    draw_line(img, x, y, x + w, y, thickness, color);
    // Bottom
    draw_line(img, x, y + h, x + w, y + h, thickness, color);
    // Left
    draw_line(img, x, y, x, y + h, thickness, color);
    // Right
    draw_line(img, x + w, y, x + w, y + h, thickness, color);
}

pub(crate) fn draw_filled_rect(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgba<u8>,
) {
    for py in y..(y + h) {
        for px in x..(x + w) {
            blend_pixel(img, px, py, color);
        }
    }
}
