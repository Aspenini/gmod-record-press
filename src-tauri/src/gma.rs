use crate::error::AppResult;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GmaFile {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn write_gma(out_path: &Path, title: &str, files: &[GmaFile]) -> AppResult<()> {
    let file = std::fs::File::create(out_path)?;
    let mut w = BufWriter::new(file);

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
    let description = serde_json::json!({
        "description": title,
        "type": "entity",
        "tags": ["fun", "realism", "roleplay"]
    })
    .to_string();
    write_cstring(&mut w, &description)?;
    write_cstring(&mut w, "Author Name")?;
    w.write_all(&1i32.to_le_bytes())?;

    for (index, entry) in files.iter().enumerate() {
        let idx = (index as u32) + 1;
        w.write_all(&idx.to_le_bytes())?;
        write_cstring(&mut w, &entry.path.replace('\\', "/"))?;
        w.write_all(&(entry.data.len() as i64).to_le_bytes())?;
        let crc = crc32fast::hash(&entry.data);
        w.write_all(&crc.to_le_bytes())?;
    }
    w.write_all(&0u32.to_le_bytes())?;

    for entry in files {
        w.write_all(&entry.data)?;
    }

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

    #[test]
    fn writes_gmad_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.gma");
        write_gma(
            &path,
            "Test Addon",
            &[GmaFile {
                path: "lua/autorun/hi.lua".into(),
                data: b"print('hi')".to_vec(),
            }],
        )
        .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"GMAD"));
        assert_eq!(bytes[4], 3);
    }
}
