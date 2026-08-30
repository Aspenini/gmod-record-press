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
    for candidate in steam_root_candidates() {
        push_root(&mut roots, candidate);
    }
    roots
}

fn push_root(roots: &mut Vec<PathBuf>, path: PathBuf) {
    if !path.is_dir() {
        return;
    }
    let path = path.canonicalize().unwrap_or(path);
    if !roots.contains(&path) {
        roots.push(path);
    }
}

fn steam_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("STEAM_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            candidates.push(PathBuf::from(dir));
        }
    }

    #[cfg(windows)]
    {
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(pf86).join("Steam"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(pf).join("Steam"));
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            candidates.push(PathBuf::from(&home).join("Steam"));
            candidates.push(PathBuf::from(home).join("AppData/Local/Steam"));
        }
        for letter in b'C'..=b'H' {
            let drive = format!("{}:\\", letter as char);
            candidates.push(PathBuf::from(&drive).join("Steam"));
            candidates.push(PathBuf::from(&drive).join("SteamLibrary"));
            candidates.push(PathBuf::from(&drive).join("Program Files (x86)/Steam"));
            candidates.push(PathBuf::from(&drive).join("Program Files/Steam"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            candidates.push(home.join("Library/Application Support/Steam"));
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = home_dir() {
            candidates.extend(linux_steam_candidates(&home));
        }
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            let xdg = PathBuf::from(xdg);
            candidates.push(xdg.join("Steam"));
        }
    }

    candidates
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_steam_candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".steam/steam"),
        home.join(".steam/root"),
        home.join(".local/share/Steam"),
        // Flatpak
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
        // Snap
        home.join("snap/steam/common/.local/share/Steam"),
        home.join("snap/steam/current/.local/share/Steam"),
    ]
}

#[cfg(not(windows))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn parse_library_folders(vdf: &Path) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(vdf).ok()?;
    let mut libs = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("\"path\"") else {
            continue;
        };
        if let Some(path) = vdf_quoted_string(rest.trim_start()) {
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

fn vdf_quoted_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next() != Some('"') {
        return None;
    }
    let mut out = String::new();
    let mut escape = false;
    for ch in chars {
        if escape {
            out.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
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
        assert!(libs[0].to_string_lossy().contains("Program Files"));
        assert_eq!(libs[1], PathBuf::from(r"D:\Games"));
    }

    #[test]
    fn parses_linux_library_vdf_paths() {
        let dir = tempfile::tempdir().unwrap();
        let vdf = dir.path().join("libraryfolders.vdf");
        std::fs::write(
            &vdf,
            r#"
"libraryfolders"
{
"0"
{
"path"		"/home/user/.local/share/Steam"
}
"1"
{
"path""/mnt/games/SteamLibrary"
}
}
"#,
        )
        .unwrap();
        let libs = parse_library_folders(&vdf).unwrap();
        assert_eq!(
            libs,
            vec![
                PathBuf::from("/home/user/.local/share/Steam"),
                PathBuf::from("/mnt/games/SteamLibrary"),
            ]
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_candidates_cover_native_flatpak_and_snap() {
        let home = PathBuf::from("/home/user");
        let candidates = linux_steam_candidates(&home);
        assert!(candidates.iter().any(|p| p.ends_with(".steam/steam")));
        assert!(candidates.iter().any(|p| p.ends_with(".local/share/Steam")));
        assert!(candidates
            .iter()
            .any(|p| p.to_string_lossy().contains("com.valvesoftware.Steam")));
        assert!(candidates
            .iter()
            .any(|p| p.to_string_lossy().contains("snap/steam")));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_detects_installed_steam_when_present() {
        let Some(home) = home_dir() else {
            return;
        };
        if linux_steam_candidates(&home).iter().all(|p| !p.is_dir()) {
            return;
        }
        assert!(
            !steam_roots().is_empty(),
            "Steam is installed but no library root was detected"
        );
    }
}
