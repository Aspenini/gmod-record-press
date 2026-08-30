use crate::error::{AppError, AppResult};
use crate::model::{AlbumProject, Track, VinylAddonInfo, VinylLibrary};
use crate::steam;
use crate::vinyl_art::{label_side, FACE_CENTERS};
use crate::vtf_encode::{decode_dxt1_vtf, encode_png, load_image, preview_data_url};
use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_vinyl_library(scan_dir: Option<String>) -> VinylLibrary {
    let gmod = steam::suggest_gmod_addons_dir();
    let dir = scan_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| gmod.as_ref().map(PathBuf::from));

    let addons = dir
        .as_ref()
        .filter(|p| p.is_dir())
        .map(|p| scan_addons_dir(p))
        .unwrap_or_default();

    VinylLibrary {
        gmod_addons_dir: gmod,
        scanned_dir: dir.map(|p| p.to_string_lossy().to_string()),
        addons,
    }
}

pub fn import_vinyl_addon(path: &str) -> AppResult<AlbumProject> {
    let root = PathBuf::from(path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "That folder was not found:\n{path}"
        )));
    }
    let parsed = parse_addon_dir(&root).ok_or_else(|| {
        AppError::Message(
            "This folder is not a Working Record Player vinyl addon.".into(),
        )
    })?;
    Ok(parsed.project)
}

fn scan_addons_dir(root: &Path) -> Vec<VinylAddonInfo> {
    let mut addons = Vec::new();
    if let Some(info) = addon_info(root) {
        addons.push(info);
        return addons;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return addons;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(info) = addon_info(&path) {
            addons.push(info);
        }
    }
    addons.sort_by(|a, b| {
        a.artist
            .to_ascii_lowercase()
            .cmp(&b.artist.to_ascii_lowercase())
            .then(a.album.to_ascii_lowercase().cmp(&b.album.to_ascii_lowercase()))
    });
    addons
}

struct ParsedAddon {
    project: AlbumProject,
    cover_path: Option<PathBuf>,
}

fn parse_addon_dir(root: &Path) -> Option<ParsedAddon> {
    let lua = read_autorun_lua(root)?;
    let vinyl = parse_register_vinyl(&lua)?;
    let id = vinyl.vinyl_id.trim();
    if id.is_empty() {
        return None;
    }

    let mat = root.join("materials/recordplayer").join(id);
    let recover = std::env::temp_dir()
        .join("gmod-record-press")
        .join("recovered")
        .join(id);

    let cover_path = png_or_vtf(&mat, "cover.png", "case_front.vtf", &recover, None);
    let back_cover_path = png_or_vtf(&mat, "back.png", "case_back.vtf", &recover, None);
    let label_path = png_or_vtf(&mat, "label.png", "vinyl.vtf", &recover, Some(crop_label));

    let meta = read_press_meta(root);
    let vinyl_color = meta
        .vinyl_color
        .or_else(|| vinyl_color_from_vtf(&mat.join("vinyl.vtf")))
        .unwrap_or_else(|| "#141414".into());
    let vinyl_resolution = meta
        .vinyl_resolution
        .filter(|w| matches!(w, 1024 | 2048 | 4096))
        .or_else(|| {
            fs::read(mat.join("vinyl.vtf"))
                .ok()
                .and_then(|bytes| decode_dxt1_vtf(&bytes).ok())
                .map(|img| img.width())
                .filter(|w| matches!(w, 1024 | 2048 | 4096))
        })
        .unwrap_or(2048);

    let sound_dir = root.join("sound/recordplayer").join(id);
    let tracks = vinyl
        .tracks
        .into_iter()
        .map(|(name, sound)| {
            let file = Path::new(&sound)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(sound.as_str());
            let path = sound_dir.join(file);
            Track {
                name,
                path: path.to_string_lossy().to_string(),
            }
        })
        .collect();

    let addon_title = read_addon_title(root).unwrap_or_default();
    let workshop_id = read_workshop_id(root);

    Some(ParsedAddon {
        project: AlbumProject {
            artist: vinyl.artist,
            album: vinyl.album,
            vinyl_id: id.to_string(),
            addon_title,
            cover_path: cover_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            back_cover_path: back_cover_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            label_path: label_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            vinyl_color,
            vinyl_resolution,
            tracks,
            workshop_id,
            workshop_description: String::new(),
            workshop_visibility: "private".into(),
            workshop_use_template: true,
        },
        cover_path,
    })
}

fn png_or_vtf(
    mat: &Path,
    png_name: &str,
    vtf_name: &str,
    recover_dir: &Path,
    transform: Option<fn(image::DynamicImage) -> image::DynamicImage>,
) -> Option<PathBuf> {
    let png = mat.join(png_name);
    if png.is_file() {
        return Some(png);
    }
    let bytes = fs::read(mat.join(vtf_name)).ok()?;
    let mut img = decode_dxt1_vtf(&bytes).ok()?;
    if let Some(transform) = transform {
        img = transform(img);
    }
    fs::create_dir_all(recover_dir).ok()?;
    let out = recover_dir.join(png_name);
    fs::write(&out, encode_png(&img).ok()?).ok()?;
    Some(out)
}

/// Cut the label back out of a vinyl sheet. The sheet is not a picture of a
/// record — the label sits on one of the two disc islands the model's UVs point
/// at, so crop there rather than at the middle of the texture. This only runs on
/// addons that shipped no `label.png` — someone else's, since this app always
/// writes one — so it stays an axis-aligned crop rather than trying to undo the
/// face's UV rotation, which those sheets will not have applied.
fn crop_label(img: image::DynamicImage) -> image::DynamicImage {
    let (w, h) = img.dimensions();
    let side = label_side(w.min(h)).max(1).min(w.min(h));
    let (cu, cv) = FACE_CENTERS[0];
    let x = ((cu * w as f32) as u32).saturating_sub(side / 2).min(w - side);
    let y = ((cv * h as f32) as u32).saturating_sub(side / 2).min(h - side);
    img.crop_imm(x, y, side, side)
}

struct PressMeta {
    vinyl_color: Option<String>,
    vinyl_resolution: Option<u32>,
}

fn read_press_meta(root: &Path) -> PressMeta {
    let empty = PressMeta {
        vinyl_color: None,
        vinyl_resolution: None,
    };
    let json = fs::read_to_string(root.join("addon.json")).ok();
    let Some(json) = json else {
        return empty;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
        return empty;
    };
    let Some(meta) = value.get("recordpress") else {
        return empty;
    };
    PressMeta {
        vinyl_color: meta
            .get("vinylColor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty()),
        vinyl_resolution: meta.get("vinylResolution").and_then(|v| v.as_u64()).map(|n| n as u32),
    }
}

fn vinyl_color_from_vtf(path: &Path) -> Option<String> {
    let img = decode_dxt1_vtf(&fs::read(path).ok()?).ok()?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let x = (w as f32 * 0.78) as u32;
    let y = h / 2;
    let pixel = img.get_pixel(x.min(w - 1), y.min(h - 1));
    Some(format!("#{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2]))
}

fn addon_info(root: &Path) -> Option<VinylAddonInfo> {
    let parsed = parse_addon_dir(root)?;
    let cover_data_url = parsed
        .cover_path
        .as_ref()
        .and_then(|path| load_image(path).ok())
        .and_then(|img| preview_data_url(&img, 160).ok());
    Some(VinylAddonInfo {
        path: root.to_string_lossy().to_string(),
        folder_name: root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("addon")
            .to_string(),
        vinyl_id: parsed.project.vinyl_id,
        artist: parsed.project.artist,
        album: parsed.project.album,
        addon_title: parsed.project.addon_title,
        track_count: parsed.project.tracks.len(),
        cover_data_url,
    })
}

fn read_autorun_lua(root: &Path) -> Option<String> {
    let autorun = root.join("lua/autorun");
    if !autorun.is_dir() {
        return None;
    }
    let mut files: Vec<PathBuf> = fs::read_dir(&autorun)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("lua"))
        })
        .collect();
    files.sort();
    for file in files {
        let text = fs::read_to_string(&file).ok()?;
        if text.contains("RegisterVinyl") {
            return Some(text);
        }
    }
    None
}

fn read_addon_title(root: &Path) -> Option<String> {
    let json = fs::read_to_string(root.join("addon.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    value
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_workshop_id(root: &Path) -> Option<u64> {
    let json = fs::read_to_string(root.join("addon.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    value.get("workshopid").and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
            .or_else(|| v.as_str()?.trim().parse().ok())
    })
}

struct ParsedVinyl {
    vinyl_id: String,
    album: String,
    artist: String,
    tracks: Vec<(String, String)>,
}

fn parse_register_vinyl(lua: &str) -> Option<ParsedVinyl> {
    let start = lua.find("RegisterVinyl")?;
    let mut cur = Cursor {
        src: lua,
        i: start + "RegisterVinyl".len(),
    };
    cur.skip_ws();
    cur.eat('(')?;
    let vinyl_id = cur.string()?;
    cur.skip_ws();
    cur.eat(',')?;
    cur.skip_ws();
    cur.eat('{')?;

    let mut album = String::new();
    let mut artist = String::new();
    let mut tracks = Vec::new();

    loop {
        cur.skip_ws();
        if cur.peek() == Some('}') {
            break;
        }
        if cur.i >= cur.src.len() {
            break;
        }
        let key = cur.ident()?;
        cur.skip_ws();
        cur.eat('=')?;
        cur.skip_ws();
        match key.as_str() {
            "name" => album = cur.string()?,
            "artist" => artist = cur.string()?,
            "tracks" => tracks = cur.track_list().unwrap_or_default(),
            _ => cur.skip_value()?,
        }
        cur.skip_ws();
        if cur.peek() == Some(',') {
            cur.i += 1;
        }
    }

    if vinyl_id.trim().is_empty() || (album.trim().is_empty() && artist.trim().is_empty()) {
        return None;
    }

    Some(ParsedVinyl {
        vinyl_id,
        album,
        artist,
        tracks,
    })
}

struct Cursor<'a> {
    src: &'a str,
    i: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        self.src.get(self.i..).unwrap_or("")
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn skip_ws(&mut self) {
        let rest = self.rest();
        let trimmed = rest.trim_start();
        self.i += rest.len() - trimmed.len();
        if self.rest().starts_with("--") {
            if let Some(n) = self.rest().find('\n') {
                self.i += n + 1;
                self.skip_ws();
            } else {
                self.i = self.src.len();
            }
        }
    }

    fn eat(&mut self, ch: char) -> Option<()> {
        if self.peek() == Some(ch) {
            self.i += ch.len_utf8();
            Some(())
        } else {
            None
        }
    }

    fn ident(&mut self) -> Option<String> {
        self.skip_ws();
        let mut chars = self.rest().chars();
        let first = chars.next()?;
        if !first.is_ascii_alphabetic() && first != '_' {
            return None;
        }
        let mut out = String::new();
        out.push(first);
        for ch in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                out.push(ch);
            } else {
                break;
            }
        }
        self.i += out.len();
        Some(out)
    }

    fn string(&mut self) -> Option<String> {
        self.skip_ws();
        let quote = self.peek()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        self.i += quote.len_utf8();
        let mut out = String::new();
        let mut chars = self.src[self.i..].chars();
        while let Some(ch) = chars.next() {
            self.i += ch.len_utf8();
            match ch {
                '\\' => {
                    let next = chars.next()?;
                    self.i += next.len_utf8();
                    out.push(match next {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                }
                c if c == quote => return Some(out),
                c => out.push(c),
            }
        }
        None
    }

    fn skip_value(&mut self) -> Option<()> {
        self.skip_ws();
        match self.peek()? {
            '"' | '\'' => {
                self.string()?;
                Some(())
            }
            '{' => self.skip_table(),
            _ => {
                while let Some(ch) = self.peek() {
                    if ch == ',' || ch == '}' {
                        break;
                    }
                    self.i += ch.len_utf8();
                }
                Some(())
            }
        }
    }

    fn skip_table(&mut self) -> Option<()> {
        self.eat('{')?;
        let mut depth = 1u32;
        while self.i < self.src.len() && depth > 0 {
            match self.peek()? {
                '"' | '\'' => {
                    self.string()?;
                }
                '{' => {
                    self.i += 1;
                    depth += 1;
                }
                '}' => {
                    self.i += 1;
                    depth -= 1;
                }
                ch => self.i += ch.len_utf8(),
            }
        }
        Some(())
    }

    fn track_list(&mut self) -> Option<Vec<(String, String)>> {
        self.skip_ws();
        self.eat('{')?;
        let mut tracks = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.i += 1;
                break;
            }
            if self.peek() != Some('{') {
                self.skip_value()?;
                self.skip_ws();
                if self.peek() == Some(',') {
                    self.i += 1;
                }
                continue;
            }
            self.i += 1;
            let mut name = String::new();
            let mut sound = String::new();
            loop {
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.i += 1;
                    break;
                }
                let key = self.ident()?;
                self.skip_ws();
                self.eat('=')?;
                self.skip_ws();
                match key.as_str() {
                    "name" => name = self.string()?,
                    "sound" => sound = self.string()?,
                    _ => self.skip_value()?,
                }
                self.skip_ws();
                if self.peek() == Some(',') {
                    self.i += 1;
                }
            }
            if !name.trim().is_empty() && !sound.trim().is_empty() {
                tracks.push((name, sound));
            }
            self.skip_ws();
            if self.peek() == Some(',') {
                self.i += 1;
            }
        }
        Some(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua::render_autorun;

    #[test]
    fn parses_generated_lua() {
        let project = AlbumProject {
            artist: "Black Sabbath".into(),
            album: "Paranoid".into(),
            vinyl_id: "paranoid".into(),
            addon_title: String::new(),
            cover_path: None,
            back_cover_path: None,
            label_path: None,
            vinyl_color: "#141414".into(),
            vinyl_resolution: 2048,
            tracks: vec![Track {
                name: r#"War Pigs / "Luke's Wall""#.into(),
                path: "x.mp3".into(),
            }],
            workshop_id: None,
            workshop_description: String::new(),
            workshop_visibility: "private".into(),
            workshop_use_template: true,
        };
        let lua = render_autorun(
            &project,
            &[(
                r#"War Pigs / "Luke's Wall""#.into(),
                "recordplayer/paranoid/war_pigs.mp3".into(),
            )],
        );
        let parsed = parse_register_vinyl(&lua).unwrap();
        assert_eq!(parsed.vinyl_id, "paranoid");
        assert_eq!(parsed.artist, "Black Sabbath");
        assert_eq!(parsed.album, "Paranoid");
        assert_eq!(parsed.tracks[0].0, r#"War Pigs / "Luke's Wall""#);
        assert!(parsed.tracks[0].1.ends_with("war_pigs.mp3"));
    }

    #[test]
    fn parses_reference_lua() {
        let lua = include_str!(
            "../../reference/working_record_player_black_sabbath_paranoid/lua/autorun/recordplayer-paranoid.lua"
        );
        let parsed = parse_register_vinyl(lua).unwrap();
        assert_eq!(parsed.vinyl_id, "paranoid");
        assert_eq!(parsed.artist, "Black Sabbath");
        assert_eq!(parsed.album, "Paranoid");
        assert_eq!(parsed.tracks.len(), 8);
        assert_eq!(parsed.tracks[0].0, "War Pigs / Luke's Wall");
    }

    #[test]
    fn imports_reference_addon_folder() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../reference/working_record_player_black_sabbath_paranoid");
        if !root.is_dir() {
            return;
        }
        let project = import_vinyl_addon(root.to_str().unwrap()).unwrap();
        assert_eq!(project.artist, "Black Sabbath");
        assert_eq!(project.album, "Paranoid");
        assert_eq!(project.vinyl_id, "paranoid");
        assert_eq!(project.tracks.len(), 8);
        assert!(project.cover_path.as_ref().unwrap().ends_with("cover.png"));
        assert!(Path::new(&project.tracks[0].path).is_file());
        assert!(project.back_cover_path.as_ref().is_some());
        assert!(project.label_path.as_ref().is_some());
        assert!(Path::new(project.back_cover_path.as_ref().unwrap()).is_file());
        assert!(Path::new(project.label_path.as_ref().unwrap()).is_file());
    }
}
