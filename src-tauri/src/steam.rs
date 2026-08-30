use std::path::{Path, PathBuf};

pub fn suggest_gmod_addons_dir() -> Option<String> {
    find_gmod_addons().map(|p| p.to_string_lossy().to_string())
}

fn find_gmod_addons() -> Option<PathBuf> {
    for steam in steam_roots() {
        if let Some(found) = addons_under_library(&steam) {
            return Some(found);
        }
        if let Some(libs) = parse_library_folders(&steam.join("steamapps/libraryfolders.vdf")) {
            for lib in libs {
                if let Some(found) = addons_under_library(&lib) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn addons_under_library(steam_or_lib: &Path) -> Option<PathBuf> {
    let addons = steam_or_lib
        .join("steamapps")
        .join("common")
        .join("GarrysMod")
        .join("garrysmod")
        .join("addons");
    if addons.is_dir() {
        Some(addons)
    } else {
        None
    }
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let push = |roots: &mut Vec<PathBuf>, p: PathBuf| {
        if p.is_dir() && !roots.contains(&p) {
            roots.push(p);
        }
    };

    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        push(&mut roots, PathBuf::from(pf86).join("Steam"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        push(&mut roots, PathBuf::from(pf).join("Steam"));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        push(&mut roots, PathBuf::from(&home).join("Steam"));
        push(
            &mut roots,
            PathBuf::from(home).join("AppData/Local/Steam"),
        );
    }

    for letter in b'C'..=b'H' {
        let drive = format!("{}:\\", letter as char);
        push(&mut roots, PathBuf::from(&drive).join("Steam"));
        push(&mut roots, PathBuf::from(&drive).join("SteamLibrary"));
        push(
            &mut roots,
            PathBuf::from(&drive).join("Program Files (x86)/Steam"),
        );
        push(
            &mut roots,
            PathBuf::from(&drive).join("Program Files/Steam"),
        );
    }

    roots
}

fn parse_library_folders(vdf: &Path) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(vdf).ok()?;
    let mut libs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("\"path\"") {
            let path = rest.trim().trim_matches('"').replace("\\\\", "\\");
            if !path.is_empty() {
                libs.push(PathBuf::from(path));
            }
        }
    }
    if libs.is_empty() {
        None
    } else {
        Some(libs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_vdf_paths() {
        let dir = tempfile::tempdir().unwrap();
        let vdf = dir.path().join("libraryfolders.vdf");
        std::fs::write(
            &vdf,
            r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
	}
	"1"
	{
		"path"		"D:\\Games"
	}
}
"#,
        )
        .unwrap();
        let libs = parse_library_folders(&vdf).unwrap();
        assert_eq!(libs.len(), 2);
        assert!(libs[0].to_string_lossy().contains("Steam"));
    }
}
