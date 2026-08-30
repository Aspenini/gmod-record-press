use crate::model::{AudioInfo, AudioScan, EmbeddedArt, TrackPicture};
use crate::slug::title_from_filename;
use crate::vtf_encode::preview_data_url;
use image::GenericImageView;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, PictureType};
use lofty::tag::{Accessor, ItemKey, Tag};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn scan_audio(paths: &[String]) -> AudioScan {
    let mut pictures: HashMap<String, EmbeddedArt> = HashMap::new();
    let mut tracks: Vec<AudioInfo> = paths.iter().filter_map(|path| {
        let p = Path::new(path);
        if !p.is_file() {
            return None;
        }
        Some(read_audio(p, &mut pictures))
    }).collect();

    tracks.sort_by(|a, b| {
        match (a.track_number, b.track_number) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.file_name.cmp(&b.file_name)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });

    AudioScan {
        tracks,
        pictures: pictures.into_values().collect(),
    }
}

fn read_audio(path: &Path, pictures: &mut HashMap<String, EmbeddedArt>) -> AudioInfo {
    let path_str = path.to_string_lossy().to_string();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("track")
        .to_string();
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);

    let mut info = AudioInfo {
        suggested_name: title_from_filename(&path_str),
        path: path_str,
        file_name,
        size,
        artist: None,
        album: None,
        album_artist: None,
        title: None,
        track_number: None,
        pictures: Vec::new(),
    };

    let Ok(tagged) = lofty::read_from_path(path) else {
        return info;
    };

    let mut disc = None;
    for tag in tagged.tags() {
        fill_text(&mut info.artist, tag.artist().as_deref());
        fill_text(&mut info.album, tag.album().as_deref());
        fill_text(&mut info.album_artist, tag.get_string(&ItemKey::AlbumArtist));
        fill_text(&mut info.title, tag.title().as_deref());
        if info.track_number.is_none() {
            info.track_number = tag.track();
        }
        if disc.is_none() {
            disc = tag.disk();
        }
        extract_pictures(tag, pictures, &mut info.pictures);
    }

    if let (Some(disc_no), Some(track_no)) = (disc, info.track_number) {
        if disc_no > 1 {
            info.track_number = Some(disc_no.saturating_mul(1000).saturating_add(track_no));
        }
    }

    if let Some(title) = info.title.as_deref() {
        if !title.is_empty() {
            info.suggested_name = title.to_string();
        }
    }

    info
}

fn fill_text(slot: &mut Option<String>, value: Option<&str>) {
    if slot.is_some() {
        return;
    }
    if let Some(text) = value.map(str::trim).filter(|s| !s.is_empty()) {
        *slot = Some(text.to_string());
    }
}

fn extract_pictures(
    tag: &Tag,
    pictures: &mut HashMap<String, EmbeddedArt>,
    refs: &mut Vec<TrackPicture>,
) {
    for picture in tag.pictures() {
        let Some(kind) = picture_kind(picture.pic_type()) else {
            continue;
        };
        let data = picture.data();
        if data.is_empty() {
            continue;
        }
        let id = picture_id(data);
        if refs.iter().any(|r| r.id == id) {
            continue;
        }

        if !pictures.contains_key(&id) {
            let Ok(img) = image::load_from_memory(data) else {
                continue;
            };
            let (width, height) = img.dimensions();
            if width < 32 || height < 32 {
                continue;
            }
            let Ok(data_url) = preview_data_url(&img, 480) else {
                continue;
            };
            let Some(path) = write_art_file(&id, data, picture.mime_type()) else {
                continue;
            };
            pictures.insert(
                id.clone(),
                EmbeddedArt {
                    id: id.clone(),
                    kind: kind.to_string(),
                    path,
                    data_url,
                    width,
                    height,
                },
            );
        } else if let Some(existing) = pictures.get_mut(&id) {
            existing.kind = preferred_kind(&existing.kind, kind).to_string();
        }

        refs.push(TrackPicture {
            id,
            kind: kind.to_string(),
        });
    }
}

fn picture_kind(pic_type: PictureType) -> Option<&'static str> {
    match pic_type {
        PictureType::CoverFront => Some("front"),
        PictureType::CoverBack => Some("back"),
        PictureType::Media => Some("label"),
        PictureType::Icon | PictureType::OtherIcon => None,
        PictureType::Other
        | PictureType::Illustration
        | PictureType::Leaflet
        | PictureType::BandLogo
        | PictureType::PublisherLogo
        | PictureType::LeadArtist
        | PictureType::Artist
        | PictureType::Band => Some("other"),
        _ => None,
    }
}

fn preferred_kind<'a>(current: &'a str, incoming: &'a str) -> &'a str {
    fn rank(kind: &str) -> u8 {
        match kind {
            "front" => 4,
            "back" => 3,
            "label" => 2,
            _ => 1,
        }
    }
    if rank(incoming) > rank(current) {
        incoming
    } else {
        current
    }
}

fn picture_id(data: &[u8]) -> String {
    format!("{:08x}{:x}", crc32fast::hash(data), data.len())
}

fn write_art_file(id: &str, data: &[u8], mime: Option<&MimeType>) -> Option<String> {
    let dir = std::env::temp_dir().join("gmod-record-press-art");
    std::fs::create_dir_all(&dir).ok()?;
    let ext = sniff_ext(data, mime);
    let path: PathBuf = dir.join(format!("{id}.{ext}"));
    if !path.is_file() {
        std::fs::write(&path, data).ok()?;
    }
    Some(path.to_string_lossy().to_string())
}

fn sniff_ext(data: &[u8], mime: Option<&MimeType>) -> &'static str {
    if let Some(mime) = mime {
        match mime {
            MimeType::Jpeg => return "jpg",
            MimeType::Png => return "png",
            MimeType::Bmp => return "bmp",
            MimeType::Gif => return "gif",
            MimeType::Tiff => return "tif",
            MimeType::Unknown(_) => {}
            _ => {}
        }
    }
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        "jpg"
    } else if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        "webp"
    } else if data.starts_with(b"BM") {
        "bmp"
    } else {
        "img"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::config::WriteOptions;
    use lofty::picture::Picture;
    use lofty::tag::{Tag, TagExt, TagType};
    use std::fs;
    use tempfile::tempdir;

    fn write_silence_wav(path: &Path) {
        let data_len: u32 = 88;
        let mut buf = Vec::new();
        buf.extend(b"RIFF");
        buf.extend(&(36 + data_len).to_le_bytes());
        buf.extend(b"WAVEfmt ");
        buf.extend(&16u32.to_le_bytes());
        buf.extend(&1u16.to_le_bytes());
        buf.extend(&1u16.to_le_bytes());
        buf.extend(&44100u32.to_le_bytes());
        buf.extend(&88200u32.to_le_bytes());
        buf.extend(&2u16.to_le_bytes());
        buf.extend(&16u16.to_le_bytes());
        buf.extend(b"data");
        buf.extend(&data_len.to_le_bytes());
        buf.extend(vec![0u8; data_len as usize]);
        fs::write(path, buf).unwrap();
    }

    fn png_bytes(r: u8, g: u8, b: u8) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(64, 64, image::Rgb([r, g, b]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn tag_wav(
        path: &Path,
        artist: &str,
        album: &str,
        album_artist: Option<&str>,
        title: &str,
        track: u32,
        pictures: Vec<(PictureType, Vec<u8>)>,
    ) {
        write_silence_wav(path);
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_artist(artist.into());
        tag.set_album(album.into());
        tag.set_title(title.into());
        tag.set_track(track);
        if let Some(album_artist) = album_artist {
            tag.insert_text(ItemKey::AlbumArtist, album_artist.into());
        }
        for (pic_type, data) in pictures {
            tag.push_picture(Picture::new_unchecked(
                pic_type,
                Some(MimeType::Png),
                None,
                data,
            ));
        }
        tag.save_to_path(path, WriteOptions::default()).unwrap();
    }

    #[test]
    fn picture_kinds_map_id3_types() {
        assert_eq!(picture_kind(PictureType::CoverFront), Some("front"));
        assert_eq!(picture_kind(PictureType::CoverBack), Some("back"));
        assert_eq!(picture_kind(PictureType::Media), Some("label"));
        assert_eq!(picture_kind(PictureType::Other), Some("other"));
        assert_eq!(picture_kind(PictureType::Icon), None);
        assert_eq!(picture_kind(PictureType::OtherIcon), None);
    }

    #[test]
    fn preferred_kind_promotes_front_cover() {
        assert_eq!(preferred_kind("other", "front"), "front");
        assert_eq!(preferred_kind("front", "back"), "front");
        assert_eq!(preferred_kind("label", "back"), "back");
    }

    #[test]
    fn reads_tags_and_prefers_title_over_filename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("01-war_pigs.wav");
        tag_wav(
            &path,
            "Black Sabbath",
            "Paranoid",
            Some("Black Sabbath"),
            "War Pigs",
            1,
            Vec::new(),
        );

        let scan = scan_audio(&[path.to_string_lossy().to_string()]);
        assert_eq!(scan.tracks.len(), 1);
        let track = &scan.tracks[0];
        assert_eq!(track.artist.as_deref(), Some("Black Sabbath"));
        assert_eq!(track.album.as_deref(), Some("Paranoid"));
        assert_eq!(track.album_artist.as_deref(), Some("Black Sabbath"));
        assert_eq!(track.title.as_deref(), Some("War Pigs"));
        assert_eq!(track.suggested_name, "War Pigs");
        assert_eq!(track.track_number, Some(1));
    }

    #[test]
    fn sorts_by_track_number() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("iron_man.wav");
        let b = dir.path().join("war_pigs.wav");
        tag_wav(&a, "Black Sabbath", "Paranoid", None, "Iron Man", 4, Vec::new());
        tag_wav(&b, "Black Sabbath", "Paranoid", None, "War Pigs", 1, Vec::new());

        let scan = scan_audio(&[
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        ]);
        assert_eq!(scan.tracks[0].title.as_deref(), Some("War Pigs"));
        assert_eq!(scan.tracks[1].title.as_deref(), Some("Iron Man"));
    }

    #[test]
    fn extracts_unique_embedded_art() {
        let dir = tempdir().unwrap();
        let cover = png_bytes(200, 20, 20);
        let other_cover = png_bytes(20, 20, 200);
        let back = png_bytes(20, 200, 20);

        let a = dir.path().join("track_a.wav");
        let b = dir.path().join("track_b.wav");
        tag_wav(
            &a,
            "Artist",
            "Album",
            None,
            "A",
            1,
            vec![
                (PictureType::CoverFront, cover.clone()),
                (PictureType::CoverBack, back.clone()),
            ],
        );
        tag_wav(
            &b,
            "Artist",
            "Album",
            None,
            "B",
            2,
            vec![(PictureType::CoverFront, other_cover.clone())],
        );

        let scan = scan_audio(&[
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        ]);
        assert_eq!(scan.pictures.len(), 3);
        assert!(scan.pictures.iter().any(|p| p.kind == "front" && Path::new(&p.path).is_file()));
        assert!(scan.pictures.iter().any(|p| p.kind == "back"));
        assert_eq!(scan.tracks[0].pictures.len(), 2);
        assert_eq!(scan.tracks[1].pictures.len(), 1);
        assert_ne!(scan.tracks[0].pictures[0].id, scan.tracks[1].pictures[0].id);
    }

    #[test]
    fn same_cover_bytes_share_one_extracted_file() {
        let dir = tempdir().unwrap();
        let cover = png_bytes(12, 34, 56);
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        tag_wav(
            &a,
            "Artist",
            "Album",
            None,
            "A",
            1,
            vec![(PictureType::CoverFront, cover.clone())],
        );
        tag_wav(
            &b,
            "Artist",
            "Album",
            None,
            "B",
            2,
            vec![(PictureType::CoverFront, cover)],
        );

        let scan = scan_audio(&[
            a.to_string_lossy().to_string(),
            b.to_string_lossy().to_string(),
        ]);
        assert_eq!(scan.pictures.len(), 1);
        assert_eq!(scan.tracks[0].pictures[0].id, scan.tracks[1].pictures[0].id);
        assert_eq!(scan.pictures[0].kind, "front");
    }

    #[test]
    fn missing_tags_fall_back_to_filename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("planet_caravan.wav");
        write_silence_wav(&path);
        let scan = scan_audio(&[path.to_string_lossy().to_string()]);
        assert_eq!(scan.tracks[0].suggested_name, "planet caravan");
        assert!(scan.tracks[0].artist.is_none());
        assert!(scan.tracks[0].album.is_none());
        assert!(scan.pictures.is_empty());
    }

    #[test]
    fn skips_missing_files() {
        let scan = scan_audio(&["Z:/definitely-not-a-real-track.mp3".into()]);
        assert!(scan.tracks.is_empty());
    }
}
