use crate::error::AppResult;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GmaFile {
    pub path: String,
    pub data: Vec<u8>,
}

/// gmad.exe always stamps this. Garry's Mod checks it and warns that an addon
/// "was created with a 3rd party tool, which might cause install/load issues"
/// when it sees anything else, so we match the official packer.
const AUTHOR: &str = "Author Name";

/// One file outside this list rejects the *whole* addon at mount time:
///     Not loading addon '...' - File not on whitelist: addon.json
///     Couldn't mount addon file '...\addon.gma' from '...'
/// A loose addons/ folder never hits this check, so the damage only shows up
/// once the addon is installed from the Workshop. Patterns come from Facepunch's
/// AddonWhiteList; `*` matches any characters, path separators included.
const WHITELIST: &[&str] = &[
    "lua/*.lua",
    "scenes/*.vcd",
    "particles/*.pcf",
    "resource/fonts/*.ttf",
    "scripts/vehicles/*.txt",
    "resource/localization/*/*.properties",
    "maps/*.bsp",
    "maps/*.lmp",
    "maps/*.nav",
    "maps/*.ain",
    "maps/thumb/*.png",
    "materials/*.vmt",
    "materials/*.vtf",
    "materials/*.png",
    "materials/*.jpg",
    "materials/*.jpeg",
    "materials/colorcorrection/*.raw",
    "models/*.mdl",
    "models/*.vtx",
    "models/*.phy",
    "models/*.ani",
    "models/*.vvd",
    "sound/*.wav",
    "sound/*.mp3",
    "sound/*.ogg",
];

/// True when Garry's Mod will mount this path out of a .gma.
pub fn is_whitelisted(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    WHITELIST
        .iter()
        .any(|pattern| wildcard_match(pattern, &path))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let v: Vec<char> = value.chars().collect();
    let (mut pi, mut vi) = (0usize, 0usize);
    let (mut star, mut star_vi) = (None, 0usize);

    while vi < v.len() {
        if pi < p.len() && p[pi] == v[vi] {
            pi += 1;
            vi += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            star_vi = vi;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            star_vi += 1;
            vi = star_vi;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Hashes everything on its way to the file so the addon CRC that closes a .gma
/// can be written without buffering the whole archive in memory.
struct CrcWriter<W: Write> {
    inner: W,
    hasher: crc32fast::Hasher,
}

impl<W: Write> CrcWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: crc32fast::Hasher::new(),
        }
    }
}

impl<W: Write> Write for CrcWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// `addon_json` is embedded verbatim as the addon description, the way gmad.exe
/// and gmpublisher both do it — Garry's Mod reads the title, type, and tags back
/// out of that string.
pub fn write_gma(
    out_path: &Path,
    title: &str,
    addon_json: &str,
    files: &[GmaFile],
) -> AppResult<()> {
    let file = std::fs::File::create(out_path)?;
    let mut w = CrcWriter::new(BufWriter::new(file));

    w.write_all(b"GMAD")?;
    w.write_all(&[3u8])?;
    w.write_all(&0u64.to_le_bytes())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    w.write_all(&ts.to_le_bytes())?;
    w.write_all(&[0u8])?; // required content
    write_cstring(&mut w, title)?;
    write_cstring(&mut w, addon_json)?;
    write_cstring(&mut w, AUTHOR)?;
    w.write_all(&1i32.to_le_bytes())?;

    // Paths are lowercased because the whitelist Garry's Mod applies at mount
    // time is lowercase-only.
    let packed: Vec<(String, &GmaFile)> = files
        .iter()
        .map(|entry| (entry.path.replace('\\', "/").to_ascii_lowercase(), entry))
        .filter(|(path, _)| is_whitelisted(path))
        .collect();

    for (index, (path, entry)) in packed.iter().enumerate() {
        let idx = (index as u32) + 1;
        w.write_all(&idx.to_le_bytes())?;
        write_cstring(&mut w, path)?;
        w.write_all(&(entry.data.len() as i64).to_le_bytes())?;
        let crc = crc32fast::hash(&entry.data);
        w.write_all(&crc.to_le_bytes())?;
    }
    w.write_all(&0u32.to_le_bytes())?;

    for (_, entry) in &packed {
        w.write_all(&entry.data)?;
    }

    // Every .gma in the wild closes with this addon CRC. Garry's Mod does not
    // verify it (gmpublisher ships zeroes here), but gmad.exe writes it and the
    // format expects the four bytes, so match them.
    let crc = w.hasher.clone().finalize();
    w.write_all(&crc.to_le_bytes())?;

    w.flush()?;
    Ok(())
}

fn write_cstring(w: &mut impl Write, s: &str) -> AppResult<()> {
    w.write_all(s.as_bytes())?;
    w.write_all(&[0u8])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample() -> Vec<GmaFile> {
        vec![
            GmaFile {
                path: "addon.json".into(),
                data: b"{}".to_vec(),
            },
            GmaFile {
                path: "lua/autorun/hi.lua".into(),
                data: b"print(1)".to_vec(),
            },
            GmaFile {
                path: "materials/recordplayer/x/cover.png".into(),
                data: b"png".to_vec(),
            },
            GmaFile {
                path: "sound/recordplayer/x/a.mp3".into(),
                data: b"mp3".to_vec(),
            },
        ]
    }

    fn packed(title: &str) -> Vec<u8> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gma");
        write_gma(&path, title, "{\"title\":\"Test Addon\"}", &sample()).unwrap();
        std::fs::read(&path).unwrap()
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn writes_gmad_header() {
        let bytes = packed("Test Addon");
        assert!(bytes.starts_with(b"GMAD"));
        assert_eq!(bytes[4], 3);
    }

    #[test]
    fn closes_with_the_addon_crc() {
        let bytes = packed("Test Addon");
        let split = bytes.len() - 4;
        let stored = u32::from_le_bytes(bytes[split..].try_into().unwrap());
        assert_eq!(stored, crc32fast::hash(&bytes[..split]));
    }

    #[test]
    fn skips_files_garrys_mod_will_not_mount() {
        let bytes = packed("Test Addon");
        assert!(!contains(&bytes, b"addon.json"));
        assert!(contains(&bytes, b"lua/autorun/hi.lua"));
        assert!(contains(&bytes, b"sound/recordplayer/x/a.mp3"));
    }

    #[test]
    fn uses_the_official_packer_author() {
        assert!(contains(&packed("Test Addon"), b"Author Name"));
    }

    #[test]
    fn whitelist_matches_nested_paths() {
        assert!(is_whitelisted("lua/autorun/recordplayer-x.lua"));
        assert!(is_whitelisted("materials/recordplayer/x/vinyl.vtf"));
        assert!(is_whitelisted("materials/recordplayer/x/cover.png"));
        assert!(is_whitelisted("sound/recordplayer/x/track.mp3"));
        assert!(!is_whitelisted("addon.json"));
        assert!(!is_whitelisted("sound/recordplayer/x/track.flac"));
        assert!(!is_whitelisted("readme.txt"));
    }
}
