use crate::vtf_encode::{cover_square, parse_hex_color};
use image::{DynamicImage, Rgba, RgbaImage};

const ICON_SIZE: u32 = 512;

/// Builds the square marketing image Steam shows for the Workshop item.
///
/// The source artwork remains recognizable on the sleeve and label, while the
/// surrounding layout makes the item read as a physical record even at Steam's
/// small card size.
pub fn render_workshop_icon(
    cover: &DynamicImage,
    label: &DynamicImage,
    vinyl_color: &str,
    artist: &str,
    album: &str,
) -> DynamicImage {
    let backdrop = cover_square(cover, ICON_SIZE).blur(24.0).to_rgba8();
    let mut canvas = RgbaImage::new(ICON_SIZE, ICON_SIZE);

    for (x, y, source) in backdrop.enumerate_pixels() {
        let vertical = y as f32 / ICON_SIZE as f32;
        let vignette_x = ((x as f32 / ICON_SIZE as f32) - 0.5).abs() * 2.0;
        let shade = (0.31 - vertical * 0.08 - vignette_x * 0.035).clamp(0.18, 0.31);
        canvas.put_pixel(
            x,
            y,
            Rgba([
                (source[0] as f32 * shade) as u8,
                (source[1] as f32 * shade) as u8,
                (source[2] as f32 * shade) as u8,
                255,
            ]),
        );
    }

    // A subtle header panel keeps metadata readable over very bright cover art.
    for y in 0..205 {
        let alpha = ((1.0 - y as f32 / 205.0) * 72.0) as u8;
        fill_row_blended(&mut canvas, y, [4, 5, 8], alpha);
    }

    draw_text_fit(&mut canvas, artist, 34, 28, 444, 1, 3, [218, 213, 200, 255]);
    let album_bottom = draw_text_fit(&mut canvas, album, 34, 67, 444, 2, 5, [250, 247, 238, 255]);
    let rule_y = (album_bottom + 17).clamp(130, 196);
    fill_rect_blended(&mut canvas, 34, rule_y, 72, 3, [222, 64, 54, 255], 235);

    // The record sits behind the sleeve, as if it has been pulled partway out.
    draw_shadow_circle(&mut canvas, 360, 350, 140);
    draw_vinyl_disc(
        &mut canvas,
        label,
        parse_hex_color(vinyl_color),
        360,
        344,
        137,
    );

    let sleeve = cover_square(cover, 236).to_rgba8();
    fill_rect_blended(&mut canvas, 25, 224, 248, 254, [0, 0, 0, 255], 105);
    fill_rect_blended(&mut canvas, 31, 218, 242, 242, [239, 235, 223, 255], 255);
    image::imageops::overlay(&mut canvas, &sleeve, 34, 221);
    draw_rect_outline(&mut canvas, 33, 220, 238, 238, [255, 255, 255, 150]);

    DynamicImage::ImageRgba8(canvas)
}

fn draw_vinyl_disc(
    canvas: &mut RgbaImage,
    label: &DynamicImage,
    base: [u8; 3],
    cx: i32,
    cy: i32,
    radius: i32,
) {
    let label_radius = (radius as f32 * 0.31).round() as i32;
    let label_art = cover_square(label, (label_radius * 2) as u32).to_rgba8();

    for y in (cy - radius - 1)..=(cy + radius + 1) {
        for x in (cx - radius - 1)..=(cx + radius + 1) {
            if x < 0 || y < 0 || x >= canvas.width() as i32 || y >= canvas.height() as i32 {
                continue;
            }
            let dx = x - cx;
            let dy = y - cy;
            let distance = ((dx * dx + dy * dy) as f32).sqrt();
            let edge_alpha = ((radius as f32 + 0.5 - distance) * 255.0).clamp(0.0, 255.0) as u8;
            if edge_alpha == 0 {
                continue;
            }

            if distance <= label_radius as f32 {
                let sx = (dx + label_radius).clamp(0, label_radius * 2 - 1) as u32;
                let sy = (dy + label_radius).clamp(0, label_radius * 2 - 1) as u32;
                let source = label_art.get_pixel(sx, sy);
                blend_pixel(
                    canvas.get_pixel_mut(x as u32, y as u32),
                    source.0,
                    edge_alpha,
                );
                continue;
            }

            let angle_highlight = ((dx - dy) as f32 / (radius as f32 * 2.0)).clamp(-0.4, 0.4);
            let groove = ((distance * 1.42).sin() * 0.055) + ((distance * 0.31).sin() * 0.025);
            let edge_darkening = 1.0 - (distance / radius as f32).powi(5) * 0.34;
            let brightness = (0.82 + angle_highlight * 0.22 + groove) * edge_darkening;
            let specular = if (dx + dy).abs() < 10 { 13.0 } else { 0.0 };
            let color = [
                (base[0] as f32 * brightness + specular).clamp(0.0, 255.0) as u8,
                (base[1] as f32 * brightness + specular).clamp(0.0, 255.0) as u8,
                (base[2] as f32 * brightness + specular).clamp(0.0, 255.0) as u8,
                255,
            ];
            blend_pixel(canvas.get_pixel_mut(x as u32, y as u32), color, edge_alpha);
        }
    }

    // Spindle hole and a small ring make the label unmistakably a record label.
    draw_circle(canvas, cx, cy, 7, [18, 18, 19, 255]);
    draw_circle(canvas, cx, cy, 3, [205, 202, 190, 255]);
}

fn draw_shadow_circle(canvas: &mut RgbaImage, cx: i32, cy: i32, radius: i32) {
    let outer = radius + 12;
    for y in (cy - outer)..=(cy + outer) {
        for x in (cx - outer)..=(cx + outer) {
            if x < 0 || y < 0 || x >= canvas.width() as i32 || y >= canvas.height() as i32 {
                continue;
            }
            let dx = x - cx;
            let dy = y - cy;
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            let alpha = ((outer as f32 - d) / 12.0 * 90.0).clamp(0.0, 90.0) as u8;
            if alpha > 0 {
                blend_pixel(
                    canvas.get_pixel_mut(x as u32, y as u32),
                    [0, 0, 0, 255],
                    alpha,
                );
            }
        }
    }
}

fn draw_text_fit(
    canvas: &mut RgbaImage,
    text: &str,
    x: u32,
    y: u32,
    max_width: u32,
    max_lines: usize,
    preferred_scale: u32,
    color: [u8; 4],
) -> u32 {
    let normalized = text.trim().to_uppercase();
    if normalized.is_empty() {
        return y;
    }

    for scale in (1..=preferred_scale).rev() {
        let lines = wrap_text(&normalized, max_width, scale);
        if lines.len() <= max_lines {
            let line_height = 9 * scale;
            for (index, line) in lines.iter().enumerate() {
                draw_text(
                    canvas,
                    line,
                    x,
                    y + index as u32 * line_height,
                    scale,
                    color,
                );
            }
            return y + lines.len() as u32 * line_height;
        }
    }

    let mut lines = wrap_text(&normalized, max_width, 1);
    lines.truncate(max_lines);
    if let Some(last) = lines.last_mut() {
        let max_chars = (max_width / 6) as usize;
        if last.chars().count() > max_chars {
            *last = last
                .chars()
                .take(max_chars.saturating_sub(3))
                .collect::<String>()
                + "...";
        }
    }
    for (index, line) in lines.iter().enumerate() {
        draw_text(canvas, line, x, y + index as u32 * 9, 1, color);
    }
    y + lines.len() as u32 * 9
}

fn wrap_text(text: &str, max_width: u32, scale: u32) -> Vec<String> {
    let max_chars = (max_width / (6 * scale)).max(1) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if word.chars().count() > max_chars {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let chars: Vec<char> = word.chars().collect();
            for chunk in chars.chunks(max_chars) {
                lines.push(chunk.iter().collect());
            }
            continue;
        }
        let next_len =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if next_len > max_chars {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn draw_text(canvas: &mut RgbaImage, text: &str, x: u32, y: u32, scale: u32, color: [u8; 4]) {
    let mut cursor = x;
    for ch in text.chars() {
        let glyph = glyph(ch);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }
                fill_rect_blended(
                    canvas,
                    cursor + col * scale,
                    y + row as u32 * scale,
                    scale,
                    scale,
                    color,
                    color[3],
                );
            }
        }
        cursor += 6 * scale;
    }
}

// Compact 5x7 display type. Uppercase gives tiny Workshop cards a stronger,
// more consistent title treatment than platform-dependent system fonts.
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [31, 4, 4, 4, 4, 4, 31],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '&' => [12, 18, 20, 8, 21, 18, 13],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        ',' => [0, 0, 0, 0, 6, 6, 4],
        '\'' => [4, 4, 2, 0, 0, 0, 0],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        ' ' => [0; 7],
        _ => [14, 17, 1, 2, 4, 0, 4],
    }
}

fn draw_circle(canvas: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if x >= 0
                && y >= 0
                && x < canvas.width() as i32
                && y < canvas.height() as i32
                && (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2)
            {
                blend_pixel(canvas.get_pixel_mut(x as u32, y as u32), color, color[3]);
            }
        }
    }
}

fn draw_rect_outline(canvas: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]) {
    fill_rect_blended(canvas, x, y, w, 1, color, color[3]);
    fill_rect_blended(canvas, x, y + h - 1, w, 1, color, color[3]);
    fill_rect_blended(canvas, x, y, 1, h, color, color[3]);
    fill_rect_blended(canvas, x + w - 1, y, 1, h, color, color[3]);
}

fn fill_row_blended(canvas: &mut RgbaImage, y: u32, color: [u8; 3], alpha: u8) {
    for x in 0..canvas.width() {
        blend_pixel(
            canvas.get_pixel_mut(x, y),
            [color[0], color[1], color[2], 255],
            alpha,
        );
    }
}

fn fill_rect_blended(
    canvas: &mut RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: [u8; 4],
    alpha: u8,
) {
    for py in y..(y + h).min(canvas.height()) {
        for px in x..(x + w).min(canvas.width()) {
            blend_pixel(canvas.get_pixel_mut(px, py), color, alpha);
        }
    }
}

fn blend_pixel(destination: &mut Rgba<u8>, source: [u8; 4], alpha: u8) {
    let a = alpha as u16;
    let inv = 255 - a;
    for channel in 0..3 {
        destination[channel] =
            ((source[channel] as u16 * a + destination[channel] as u16 * inv) / 255) as u8;
    }
    destination[3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_icon_contains_metadata_sleeve_and_disc() {
        let cover =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([30, 80, 180, 255])));
        let label =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([220, 50, 40, 255])));
        let icon =
            render_workshop_icon(&cover, &label, "#202020", "Test Artist", "Demo Days").to_rgba8();

        assert_eq!((icon.width(), icon.height()), (512, 512));
        assert!(
            icon.get_pixel(40, 30)[0] > 150,
            "artist text should be visible"
        );
        assert!(
            icon.get_pixel(80, 280)[2] > 120,
            "front sleeve should retain cover art"
        );
        assert!(
            icon.get_pixel(360, 344)[0] > 130,
            "disc should use the label art"
        );
        assert!(
            icon.get_pixel(470, 344)[0] < 80,
            "disc rim should use the vinyl colour"
        );
    }

    #[test]
    fn long_titles_wrap_without_leaving_the_canvas() {
        let image = DynamicImage::new_rgb8(32, 32);
        let icon = render_workshop_icon(
            &image,
            &image,
            "#111111",
            "A Very Long Collaboration Featuring Several Artists",
            "The Unnecessarily Long Album Name Deluxe Anniversary Edition",
        );
        assert_eq!((icon.width(), icon.height()), (512, 512));
    }
}
