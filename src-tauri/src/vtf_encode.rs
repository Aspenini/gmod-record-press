use crate::error::{AppError, AppResult};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat, RgbaImage};
use std::io::Cursor;
use std::path::Path;
use texpresso::{Algorithm, Format, Params};

const IMAGE_FORMAT_DXT1: i32 = 13;

pub fn load_image(path: &Path) -> AppResult<DynamicImage> {
    Ok(image::open(path)?)
}

pub fn cover_square(img: &DynamicImage, size: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return DynamicImage::new_rgba8(size, size);
    }
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop_imm(x, y, side, side)
        .resize_exact(size, size, FilterType::Lanczos3)
}

pub fn fit_max_edge(img: &DynamicImage, max_edge: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= max_edge {
        return img.clone();
    }
    img.resize(max_edge, max_edge, FilterType::Lanczos3)
}

pub fn encode_png(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
    Ok(buf)
}

pub fn encode_workshop_jpeg(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let square = cover_square(img, 512).to_rgb8();
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90).write_image(
        square.as_raw(),
        square.width(),
        square.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf)
}

pub fn encode_dxt1_vtf(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || !w.is_power_of_two() || !h.is_power_of_two() {
        return Err(AppError::Message(format!(
            "VTF textures must be a power of two (got {w}x{h})."
        )));
    }

    let mut mip_images = Vec::new();
    let mut current = img.to_rgba8();
    loop {
        let cw = current.width();
        let ch = current.height();
        mip_images.push(compress_bc1(&current));
        if cw == 1 && ch == 1 {
            break;
        }
        let nw = (cw / 2).max(1);
        let nh = (ch / 2).max(1);
        current = image::imageops::resize(&current, nw, nh, FilterType::Triangle);
    }

    // VTF stores mipmaps smallest → largest.
    mip_images.reverse();

    let low_w: u8 = 16.min(w as u8).max(1);
    let low_h: u8 = 16.min(h as u8).max(1);
    let low = image::imageops::resize(&img.to_rgba8(), low_w as u32, low_h as u32, FilterType::Triangle);
    let lowres = compress_bc1(&low);

    let mipmap_count = mip_images.len() as u8;
    let mut data = Vec::new();
    write_header_72(&mut data, w as u16, h as u16, mipmap_count, low_w, low_h);
    data.extend_from_slice(&lowres);
    for mip in mip_images {
        data.extend_from_slice(&mip);
    }
    Ok(data)
}

fn write_header_72(data: &mut Vec<u8>, width: u16, height: u16, mips: u8, low_w: u8, low_h: u8) {
    data.extend_from_slice(b"VTF\0");
    data.extend_from_slice(&7u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&80u32.to_le_bytes());
    data.extend_from_slice(&width.to_le_bytes());
    data.extend_from_slice(&height.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes()); // flags
    data.extend_from_slice(&1u16.to_le_bytes()); // frames
    data.extend_from_slice(&0u16.to_le_bytes()); // first frame
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&0f32.to_le_bytes());
    data.extend_from_slice(&0f32.to_le_bytes());
    data.extend_from_slice(&0f32.to_le_bytes());
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&1f32.to_le_bytes()); // bumpmap scale
    data.extend_from_slice(&IMAGE_FORMAT_DXT1.to_le_bytes());
    data.push(mips);
    data.extend_from_slice(&IMAGE_FORMAT_DXT1.to_le_bytes());
    data.push(low_w);
    data.push(low_h);
    data.extend_from_slice(&1u16.to_le_bytes()); // depth
    while data.len() < 80 {
        data.push(0);
    }
}

fn compress_bc1(img: &RgbaImage) -> Vec<u8> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let (cw, ch, pixels) = if w < 4 || h < 4 {
        let mut padded = vec![0u8; 4 * 4 * 4];
        for y in 0..h {
            for x in 0..w {
                let p = img.get_pixel(x as u32, y as u32).0;
                let i = (y * 4 + x) * 4;
                padded[i..i + 4].copy_from_slice(&p);
            }
        }
        (4usize, 4usize, padded)
    } else {
        (w, h, img.as_raw().clone())
    };

    let params = Params {
        algorithm: Algorithm::RangeFit,
        weights: [0.2126, 0.7152, 0.0722],
        weigh_colour_by_alpha: false,
    };
    let mut out = vec![0u8; Format::Bc1.compressed_size(cw, ch)];
    Format::Bc1.compress(&pixels, cw, ch, params, &mut out);
    out
}

pub fn preview_data_url(img: &DynamicImage, max_edge: u32) -> AppResult<String> {
    let fitted = fit_max_edge(img, max_edge).to_rgb8();
    let mut buf = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 82).write_image(
        fitted.as_raw(),
        fitted.width(),
        fitted.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf)
    ))
}

pub fn parse_hex_color(hex: &str) -> [u8; 3] {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return [r, g, b];
        }
    }
    [20, 20, 20]
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn vtf_header_is_valid_dxt1() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            32,
            32,
            image::Rgba([200, 40, 40, 255]),
        ));
        let bytes = encode_dxt1_vtf(&img).expect("encode");
        assert!(bytes.starts_with(b"VTF\0"));
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 80);
        let width = u16::from_le_bytes([bytes[16], bytes[17]]);
        let height = u16::from_le_bytes([bytes[18], bytes[19]]);
        let format = i32::from_le_bytes(bytes[52..56].try_into().unwrap());
        assert_eq!(width, 32);
        assert_eq!(height, 32);
        assert_eq!(format, 13);
        assert!(bytes.len() > 80);
    }
}
