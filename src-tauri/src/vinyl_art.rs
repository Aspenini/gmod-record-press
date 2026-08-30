use crate::vtf_encode::{cover_square, parse_hex_color};
use image::{DynamicImage, Rgba, RgbaImage};

/// Label radius as a fraction of the disc radius, measured against the
/// official Paranoid vinyl (center sticker occupies ~35% of the diameter).
const LABEL_RATIO: f32 = 0.35;
const HOLE_RATIO: f32 = 0.035;
const RIM_RATIO: f32 = 0.985;

pub fn render_vinyl(label: &DynamicImage, color_hex: &str, size: u32) -> DynamicImage {
    let size = size.max(64);
    let label_sq = cover_square(label, size).to_rgba8();
    let vinyl = parse_hex_color(color_hex);
    let mut buf = RgbaImage::new(size, size);

    let cx = (size as f32 - 1.0) * 0.5;
    let cy = cx;
    let radius = size as f32 * 0.5;
    let label_r = radius * LABEL_RATIO;
    let hole_r = radius * HOLE_RATIO;
    let rim_r = radius * RIM_RATIO;
    let groove_inner = label_r + radius * 0.018;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let r = (dx * dx + dy * dy).sqrt();
            let pixel = sample_pixel(
                dx,
                dy,
                r,
                radius,
                label_r,
                hole_r,
                rim_r,
                groove_inner,
                vinyl,
                &label_sq,
                size,
            );
            buf.put_pixel(x, y, pixel);
        }
    }

    DynamicImage::ImageRgba8(buf)
}

fn sample_pixel(
    dx: f32,
    dy: f32,
    r: f32,
    radius: f32,
    label_r: f32,
    hole_r: f32,
    rim_r: f32,
    groove_inner: f32,
    vinyl: [u8; 3],
    label: &image::RgbaImage,
    size: u32,
) -> Rgba<u8> {
    if r > rim_r {
        return Rgba([0, 0, 0, 255]);
    }

    // Soft outer lip.
    if r > rim_r - radius * 0.012 {
        let edge = shade(vinyl, 1.25);
        return Rgba([edge[0], edge[1], edge[2], 255]);
    }

    if r <= hole_r {
        return Rgba([6, 6, 7, 255]);
    }

    // Spindle ring.
    if r <= hole_r + radius * 0.012 {
        let metal = shade(vinyl, 1.8);
        return Rgba([metal[0].max(70), metal[1].max(70), metal[2].max(70), 255]);
    }

    if r <= label_r {
        let paper_ring = label_r - radius * 0.012;
        if r > paper_ring {
            return Rgba([236, 228, 210, 255]);
        }
        return sample_label(dx, dy, label_r, label, size);
    }

    // Dead wax between label and grooves.
    if r < groove_inner {
        let c = shade(vinyl, 0.82);
        return with_sheen(c, dx, dy, r, radius);
    }

    let groove = ((r * 0.42).sin() * 0.5 + 0.5) * 0.18;
    let runout = if r > rim_r - radius * 0.055 { 0.08 } else { 0.0 };
    let mix = 0.78 + groove + runout;
    let c = shade(vinyl, mix);
    with_sheen(c, dx, dy, r, radius)
}

fn sample_label(
    dx: f32,
    dy: f32,
    label_r: f32,
    label: &image::RgbaImage,
    size: u32,
) -> Rgba<u8> {
    let u = ((dx / label_r) * 0.5 + 0.5).clamp(0.0, 1.0);
    let v = ((dy / label_r) * 0.5 + 0.5).clamp(0.0, 1.0);
    let sx = (u * (size.saturating_sub(1) as f32)).round() as u32;
    let sy = (v * (size.saturating_sub(1) as f32)).round() as u32;
    let p = label.get_pixel(sx.min(label.width() - 1), sy.min(label.height() - 1));
    Rgba([p[0], p[1], p[2], 255])
}

fn shade(rgb: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (rgb[0] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 * factor).round().clamp(0.0, 255.0) as u8,
    ]
}

fn with_sheen(rgb: [u8; 3], dx: f32, dy: f32, r: f32, radius: f32) -> Rgba<u8> {
    let nx = dx / radius;
    let ny = dy / radius;
    let light = (nx * 0.35 - ny * 0.7 + 0.12).clamp(0.0, 1.0);
    let sheen = light.powf(4.5) * 70.0;
    let ring = ((r / radius) * 80.0).sin().abs() * 6.0;
    Rgba([
        (rgb[0] as f32 + sheen + ring).clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 + sheen + ring).clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 + sheen + ring).clamp(0.0, 255.0) as u8,
        255,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vinyl_has_label_in_the_center() {
        let label = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            64,
            Rgba([220, 30, 30, 255]),
        ));
        let img = render_vinyl(&label, "#121212", 128).to_rgba8();
        let label_px = img.get_pixel(76, 64);
        assert!(
            label_px[0] > 150,
            "label ring should sample the red sticker, got {label_px:?}"
        );
        let corner = img.get_pixel(0, 0);
        assert!(corner[0] < 20 && corner[1] < 20);
    }
}
