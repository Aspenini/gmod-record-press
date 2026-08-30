use crate::vtf_encode::{cover_square, parse_hex_color};
use image::{DynamicImage, RgbaImage};
use rayon::prelude::*;

// Layout of `models/recordplayer/vinyl.mdl` from the Working Record Player addon,
// read out of the model's .vvd.
//
// The record is a flat disc of radius 7.5044 units and each face's UVs are an
// exact conformal map of the disc plane — a rotation on the -Z face, a reflection
// on the +Z face, both at scale 0.0386880 uv/unit and both fitting the vertices
// with zero error. So the base texture is *not* a picture of a record. It is two
// disc islands sitting in an otherwise unused sheet:
//
//   -Z face: centre (0.709158, 0.291208), radius 0.290332
//   +Z face: centre (0.290675, 0.709352), radius 0.290332
//
// which is exactly how the addon's own `models/textures/vinyl.vtf` is laid out.
// Painting a record centred on the texture — as this used to — puts the label
// just off the edge of both islands, so only a crescent of it lands on the disc,
// out by the rim.
//
// Grooves come from the shared `models/textures/vinyl_n` bumpmap that every vinyl
// material references, so they are already aligned to the real disc centre and
// must not be painted here as well.
//
// The islands are not axis-aligned to the sheet: the -Z face's UVs are rotated
// 25.3 degrees and the +Z face's are a reflection. Art pasted square onto the
// sheet therefore lands on the record askew — the addon's own texture is like
// this, which its radially symmetric swirl label hides. Sampling through the
// inverse of each face's map instead puts the label upright in model space, so
// it reads the same way up as the sleeve art and matches on both faces.

/// Disc radius in UV, for both faces.
const DISC_UV_RADIUS: f32 = 0.290332;

/// UV per model unit, the scale of both faces' maps.
const UV_SCALE: f32 = 0.0386880;

/// Label ring in the mesh sits at r = 2.4499 of the 7.5044 disc radius.
pub const LABEL_RATIO: f32 = 0.32646;

/// Label radius in UV.
pub const LABEL_UV_RADIUS: f32 = DISC_UV_RADIUS * LABEL_RATIO;

/// Disc centres in UV, one per record face.
pub const FACE_CENTERS: [(f32, f32); 2] = [(0.709158, 0.291208), (0.290675, 0.709352)];

struct Face {
    /// Disc centre in UV.
    center: (f32, f32),
    /// Row-major `M`, where `uv = M * (x, y) + centre` for a point on the face.
    /// `M` is conformal, so its inverse is just `Mᵀ / UV_SCALE²`.
    m: [f32; 4],
    /// The face points along -Z, so it is read from the other side: mirror the
    /// art back or it comes out reversed.
    mirrored: bool,
}

const FACES: [Face; 2] = [
    Face {
        center: FACE_CENTERS[0],
        m: [0.0349735, 0.0165413, -0.0165413, 0.0349735],
        mirrored: true,
    },
    Face {
        center: FACE_CENTERS[1],
        m: [0.0094004, 0.0375286, 0.0375286, -0.0094004],
        mirrored: false,
    },
];

pub fn render_vinyl(label: &DynamicImage, color_hex: &str, size: u32) -> DynamicImage {
    let size = size.max(64);
    let vinyl = parse_hex_color(color_hex);

    // The label only ever covers `2 * LABEL_UV_RADIUS` of the sheet, so this is
    // all the sticker resolution the texture can hold.
    let label_px = label_side(size).max(8);
    let sticker = cover_square(label, label_px).to_rgba8();

    let texel = 1.0 / size as f32;
    let row_bytes = size as usize * 4;
    let mut buf = vec![0u8; size as usize * row_bytes];

    buf.par_chunks_mut(row_bytes)
        .enumerate()
        .for_each(|(y, row)| {
            let v = (y as f32 + 0.5) * texel;
            for x in 0..size {
                let u = (x as f32 + 0.5) * texel;
                let pixel = shade_texel(u, v, texel, vinyl, &sticker);
                let i = x as usize * 4;
                row[i] = pixel[0];
                row[i + 1] = pixel[1];
                row[i + 2] = pixel[2];
                row[i + 3] = 255;
            }
        });

    DynamicImage::ImageRgba8(
        RgbaImage::from_raw(size, size, buf).expect("vinyl buffer matches image size"),
    )
}

/// Side of the label square inside a `size`-wide vinyl sheet.
pub fn label_side(size: u32) -> u32 {
    (LABEL_UV_RADIUS * 2.0 * size as f32).round() as u32
}

fn shade_texel(u: f32, v: f32, texel: f32, vinyl: [u8; 3], sticker: &RgbaImage) -> [u8; 3] {
    // Undoing the face map costs one divide by UV_SCALE² and one by the label
    // radius in model units; folded together that is a single constant.
    let to_label = 1.0 / (UV_SCALE * LABEL_UV_RADIUS);

    for face in &FACES {
        let du = u - face.center.0;
        let dv = v - face.center.1;
        let r = (du * du + dv * dv).sqrt();
        if r > LABEL_UV_RADIUS + texel {
            continue;
        }
        // Model-space position, scaled so the label spans -1..1.
        let x = (face.m[0] * du + face.m[2] * dv) * to_label;
        let y = (face.m[1] * du + face.m[3] * dv) * to_label;
        let art = sample_label(sticker, if face.mirrored { -x } else { x }, -y);
        // Feather the last texel so the sticker edge does not stair-step.
        let cover = 1.0 - smoothstep(LABEL_UV_RADIUS - texel, LABEL_UV_RADIUS + texel, r);
        return [
            mix(vinyl[0], art[0], cover),
            mix(vinyl[1], art[1], cover),
            mix(vinyl[2], art[2], cover),
        ];
    }
    vinyl
}

/// Bilinear sample of the sticker, with `lx`/`ly` running -1..1 across it.
fn sample_label(sticker: &RgbaImage, lx: f32, ly: f32) -> [u8; 3] {
    let w = sticker.width();
    let h = sticker.height();
    let fx = ((lx * 0.5 + 0.5) * (w - 1) as f32).clamp(0.0, (w - 1) as f32);
    let fy = ((ly * 0.5 + 0.5) * (h - 1) as f32).clamp(0.0, (h - 1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let mut out = [0u8; 3];
    for (c, channel) in out.iter_mut().enumerate() {
        let top = lerp(
            sticker.get_pixel(x0, y0)[c] as f32,
            sticker.get_pixel(x1, y0)[c] as f32,
            tx,
        );
        let bottom = lerp(
            sticker.get_pixel(x0, y1)[c] as f32,
            sticker.get_pixel(x1, y1)[c] as f32,
            tx,
        );
        *channel = lerp(top, bottom, ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix(base: u8, over: u8, t: f32) -> u8 {
    lerp(base as f32, over as f32, t).round().clamp(0.0, 255.0) as u8
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge1 <= edge0 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn red_label() -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            64,
            Rgba([220, 30, 30, 255]),
        ))
    }

    #[test]
    fn label_lands_on_both_disc_islands() {
        let size = 512u32;
        let img = render_vinyl(&red_label(), "#121212", size).to_rgba8();
        for (cu, cv) in FACE_CENTERS {
            let x = (cu * size as f32) as u32;
            let y = (cv * size as f32) as u32;
            let px = img.get_pixel(x, y);
            assert!(
                px[0] > 150 && px[1] < 90,
                "face centre ({cu}, {cv}) should be sticker, got {px:?}"
            );
        }
    }

    #[test]
    fn sheet_outside_the_labels_is_the_vinyl_colour() {
        let size = 512u32;
        let img = render_vinyl(&red_label(), "#121212", size).to_rgba8();
        // The middle of the sheet is between the two islands, and a corner is off
        // the model entirely — both are plain vinyl, never sticker.
        for (x, y) in [(size / 2, size / 2), (0, 0), (size - 1, size - 1)] {
            let px = img.get_pixel(x, y);
            assert_eq!([px[0], px[1], px[2]], [0x12, 0x12, 0x12], "at ({x}, {y})");
        }
    }

    #[test]
    fn label_stops_at_the_ring_in_the_mesh() {
        let size = 1024u32;
        let img = render_vinyl(&red_label(), "#121212", size).to_rgba8();
        let (cu, cv) = FACE_CENTERS[0];
        let cx = cu * size as f32;
        let cy = cv * size as f32;
        let r = LABEL_UV_RADIUS * size as f32;

        let inside = img.get_pixel((cx + r * 0.9) as u32, cy as u32);
        assert!(inside[0] > 150, "just inside the label, got {inside:?}");
        let outside = img.get_pixel((cx + r * 1.1) as u32, cy as u32);
        assert!(outside[0] < 40, "just outside the label, got {outside:?}");
    }

    #[test]
    fn matches_the_official_vinyl_texture_layout() {
        // The addon's own Paranoid vinyl.vtf carries its labels at these islands;
        // measuring our sheet the same way has to land in the same place.
        let size = 1024u32;
        let img = render_vinyl(&red_label(), "#000000", size).to_rgba8();
        let (mut x0, mut y0, mut x1, mut y1) = (size, size, 0u32, 0u32);
        for (x, y, px) in img.enumerate_pixels() {
            if px[0] > 60 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
        // Union bounding box of both islands, as fractions of the sheet.
        let expect = |v: f32| (v * size as f32).round() as u32;
        let low = expect(FACE_CENTERS[1].0 - LABEL_UV_RADIUS);
        let high = expect(FACE_CENTERS[0].0 + LABEL_UV_RADIUS);
        assert!(x0.abs_diff(low) <= 2 && y0.abs_diff(low) <= 2, "{x0} {y0}");
        assert!(x1.abs_diff(high) <= 2 && y1.abs_diff(high) <= 2, "{x1} {y1}");
    }

    #[test]
    fn label_sits_upright_on_both_faces() {
        // Red in the top-left quadrant of the sticker only.
        let mut art = image::RgbaImage::from_pixel(64, 64, Rgba([10, 10, 10, 255]));
        for y in 0..32 {
            for x in 0..32 {
                art.put_pixel(x, y, Rgba([230, 20, 20, 255]));
            }
        }
        let size = 1024u32;
        let img = render_vinyl(&DynamicImage::ImageRgba8(art), "#121212", size).to_rgba8();

        let label_r = LABEL_UV_RADIUS / UV_SCALE;
        let at = |face: &Face, x: f32, y: f32| {
            let (x, y) = (x * label_r, y * label_r);
            let u = face.m[0] * x + face.m[1] * y + face.center.0;
            let v = face.m[2] * x + face.m[3] * y + face.center.1;
            *img.get_pixel((u * size as f32) as u32, (v * size as f32) as u32)
        };

        // Top-left as each face's own viewer sees it: +y is up on both, and the
        // -Z face is looked at from behind, so its viewer's left is model +x.
        for (face, left) in [(&FACES[0], 0.5f32), (&FACES[1], -0.5f32)] {
            let corner = at(face, left, 0.5);
            assert!(corner[0] > 150, "label top-left should be red, got {corner:?}");
            let opposite = at(face, -left, -0.5);
            assert!(
                opposite[0] < 60,
                "label bottom-right should be dark, got {opposite:?}"
            );
        }
    }

    #[test]
    fn official_vinyl_texture_carries_its_labels_at_our_islands() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../reference/working_record_player_black_sabbath_paranoid/materials/recordplayer/paranoid/vinyl.vtf",
        );
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        let img = crate::vtf_encode::decode_dxt1_vtf(&bytes)
            .expect("decode official vinyl")
            .to_luma8();
        let (w, h) = img.dimensions();

        let disc_mean = |cu: f32, cv: f32| {
            let (mut sum, mut n) = (0u64, 0u64);
            for (x, y, px) in img.enumerate_pixels() {
                let du = (x as f32 + 0.5) / w as f32 - cu;
                let dv = (y as f32 + 0.5) / h as f32 - cv;
                if du * du + dv * dv <= LABEL_UV_RADIUS * LABEL_UV_RADIUS {
                    sum += px[0] as u64;
                    n += 1;
                }
            }
            sum as f32 / n.max(1) as f32
        };

        for (cu, cv) in FACE_CENTERS {
            let mean = disc_mean(cu, cv);
            assert!(mean > 100.0, "island ({cu}, {cv}) mean luma {mean}");
        }
        let middle = disc_mean(0.5, 0.5);
        assert!(middle < 8.0, "sheet middle is bare vinyl, mean luma {middle}");
    }

    #[test]
    fn label_side_tracks_the_sheet() {
        assert_eq!(label_side(4096), 776);
        assert_eq!(label_side(2048), 388);
    }
}
