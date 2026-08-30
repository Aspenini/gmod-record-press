use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumProject {
    pub artist: String,
    pub album: String,
    pub vinyl_id: String,
    #[serde(default)]
    pub addon_title: String,
    pub cover_path: Option<String>,
    pub back_cover_path: Option<String>,
    pub label_path: Option<String>,
    #[serde(default = "default_vinyl_color")]
    pub vinyl_color: String,
    #[serde(default = "default_vinyl_resolution")]
    pub vinyl_resolution: u32,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub workshop_id: Option<u64>,
    #[serde(default)]
    pub workshop_description: String,
    #[serde(default = "default_workshop_visibility")]
    pub workshop_visibility: String,
    #[serde(default = "default_true")]
    pub workshop_use_template: bool,
}

fn default_vinyl_color() -> String {
    "#141414".to_string()
}

fn default_vinyl_resolution() -> u32 {
    2048
}

fn default_workshop_visibility() -> String {
    "private".to_string()
}

fn default_true() -> bool {
    true
}

impl AlbumProject {
    pub fn resolved_title(&self) -> String {
        standard_addon_title(&self.artist, &self.album)
    }

    pub fn addon_folder_name(&self) -> String {
        format!("recordplayer_{}", self.vinyl_id)
    }
}

pub fn standard_addon_title(artist: &str, album: &str) -> String {
    let artist = artist.trim();
    let album = album.trim();
    match (artist.is_empty(), album.is_empty()) {
        (false, false) => format!("[Working Record Player] {artist} - {album}"),
        (false, true) => format!("[Working Record Player] {artist}"),
        (true, false) => format!("[Working Record Player] {album}"),
        (true, true) => "[Working Record Player]".to_string(),
    }
}

pub fn standard_workshop_description(artist: &str, album: &str) -> String {
    let artist = artist.trim();
    let album = album.trim();
    let pack = match (artist.is_empty(), album.is_empty()) {
        (false, false) => format!("{artist} - {album}"),
        (false, true) => artist.to_string(),
        (true, false) => album.to_string(),
        (true, true) => "Music Pack".to_string(),
    };
    format!(
        "[h1]Working Record Player - {pack}[/h1]\n\
         \n\
         Adds music for use with the [b]Working Record Player[/b] addon in Garry's Mod.\n\
         \n\
         [h2]Copyright Notice[/h2]\n\
         \n\
         This is an unofficial, fan-made Workshop addon.\n\
         \n\
         I do not claim ownership of any music, recordings, artwork, trademarks, or other copyrighted material included in this addon. All rights belong to their respective artists, labels, publishers, and copyright holders.\n\
         \n\
         This addon is not affiliated with or endorsed by the original artists or rights holders and is provided for entertainment purposes only."
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub dest_dir: String,
    #[serde(default)]
    pub write_gma: bool,
    #[serde(default)]
    pub write_workshop_icon: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub stage: String,
    pub detail: String,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub addon_dir: String,
    pub gma_path: Option<String>,
    pub workshop_icon_path: Option<String>,
    pub files_written: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VinylAddonInfo {
    pub path: String,
    pub folder_name: String,
    pub vinyl_id: String,
    pub artist: String,
    pub album: String,
    pub addon_title: String,
    pub track_count: usize,
    pub cover_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VinylLibrary {
    pub gmod_addons_dir: Option<String>,
    pub scanned_dir: Option<String>,
    pub addons: Vec<VinylAddonInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopStatus {
    pub connected: bool,
    pub persona: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItem {
    pub id: u64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopPublishOptions {
    #[serde(default)]
    pub dest_dir: String,
    #[serde(default)]
    pub workshop_id: Option<u64>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_workshop_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub change_note: String,
    #[serde(default = "default_true")]
    pub use_template: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopProgress {
    pub stage: String,
    pub detail: String,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopPublishResult {
    pub workshop_id: u64,
    pub url: String,
    pub updated: bool,
    pub needs_legal_agreement: bool,
    pub legal_agreement_url: String,
    pub export: ExportResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreview {
    pub path: String,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    pub path: String,
    pub file_name: String,
    pub suggested_name: String,
    pub size: u64,
}

pub fn validate_project(project: &AlbumProject) -> Vec<Issue> {
    let mut issues = Vec::new();

    if project.artist.trim().is_empty() {
        issues.push(error("Artist is required."));
    }
    if project.album.trim().is_empty() {
        issues.push(error("Album title is required."));
    }

    let id = project.vinyl_id.trim();
    if id.is_empty() {
        issues.push(error("Vinyl ID is required."));
    } else if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        issues.push(error(
            "Vinyl ID must be lowercase letters, numbers, and underscores.",
        ));
    } else if id.chars().all(|c| c == '_') {
        issues.push(error("Vinyl ID needs at least one letter or number."));
    }

    if project.cover_path.as_ref().map(|s| s.trim().is_empty()) != Some(false) {
        issues.push(error("Front cover artwork is required."));
    } else if let Some(path) = &project.cover_path {
        if !std::path::Path::new(path).is_file() {
            issues.push(error(format!("Front cover was not found:\n{path}")));
        }
    }

    if let Some(path) = &project.back_cover_path {
        if !path.trim().is_empty() && !std::path::Path::new(path).is_file() {
            issues.push(error(format!("Back cover was not found:\n{path}")));
        }
    } else {
        issues.push(warning("No back cover — the front cover will be used."));
    }

    if project.label_path.as_ref().map(|s| !s.trim().is_empty()) != Some(true) {
        issues.push(warning("No custom vinyl label — the front cover will be used."));
    }

    match project.vinyl_resolution {
        1024 | 2048 | 4096 => {}
        other => issues.push(error(format!(
            "Vinyl resolution must be 1024, 2048, or 4096 (got {other})."
        ))),
    }

    if project.tracks.is_empty() {
        issues.push(error("Add at least one track."));
    }

    for (index, track) in project.tracks.iter().enumerate() {
        let n = index + 1;
        if track.name.trim().is_empty() {
            issues.push(error(format!("Track {n} needs a name.")));
        }
        if track.path.trim().is_empty() {
            issues.push(error(format!("Track {n} has no audio file.")));
        } else if !std::path::Path::new(&track.path).is_file() {
            issues.push(error(format!(
                "Track {n} audio was not found:\n{}",
                track.path
            )));
        } else {
            let ext = std::path::Path::new(&track.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(ext.as_str(), "mp3" | "ogg" | "wav") {
                issues.push(warning(format!(
                    "Track {n} is .{ext} — GMod is happiest with mp3, ogg, or wav."
                )));
            }
        }
    }

    issues
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.level == "error")
}

fn error(message: impl Into<String>) -> Issue {
    Issue {
        level: "error".into(),
        message: message.into(),
    }
}

fn warning(message: impl Into<String>) -> Issue {
    Issue {
        level: "warning".into(),
        message: message.into(),
    }
}
