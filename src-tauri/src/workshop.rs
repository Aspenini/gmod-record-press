use crate::error::{AppError, AppResult};
use crate::export::export_album;
use crate::model::{
    standard_workshop_description, AlbumProject, ExportOptions, ExportProgress, ExportResult,
    WorkshopItem, WorkshopPublishOptions, WorkshopPublishResult, WorkshopProgress, WorkshopStatus,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use steamworks::{
    sys, AppId, AppIDs, Client, FileType, PublishedFileId, PublishedFileVisibility, SteamError,
    UGCType, UpdateStatus, UserList, UserListOrder,
};

pub const GMOD_APP_ID: AppId = AppId(4000);
pub const RECORD_PLAYER_WORKSHOP_ID: u64 = 3777821069;
const LEGAL_AGREEMENT_URL: &str = "https://steamcommunity.com/workshop/workshoplegalagreement";
const TITLE_MAX: usize = 128;
const DESCRIPTION_MAX: usize = 8000;

static STEAM: Mutex<Option<Client>> = Mutex::new(None);
static CALLBACKS: OnceLock<()> = OnceLock::new();

pub fn status() -> WorkshopStatus {
    match connect() {
        Ok(client) => WorkshopStatus {
            connected: true,
            persona: Some(client.friends().name()),
            error: None,
        },
        Err(err) => WorkshopStatus {
            connected: false,
            persona: None,
            error: Some(err.to_string()),
        },
    }
}

pub fn list_my_items() -> AppResult<Vec<WorkshopItem>> {
    let client = connect()?;
    let account = client.user().steam_id().account_id();
    let (tx, rx) = mpsc::channel();
    client
        .ugc()
        .query_user(
            account,
            UserList::Published,
            UGCType::ItemsReadyToUse,
            UserListOrder::LastUpdatedDesc,
            AppIDs::ConsumerAppId(GMOD_APP_ID),
            1,
        )
        .map_err(|e| AppError::Message(format!("Could not query Workshop items: {e}")))?
        .require_tag("addon")
        .fetch(move |result| {
            let items = result.map(|data| {
                data.iter()
                    .flatten()
                    .map(|item| WorkshopItem {
                        id: item.published_file_id.0,
                        title: item.title,
                    })
                    .collect::<Vec<_>>()
            });
            let _ = tx.send(items);
        });

    match recv_timeout(&rx, Duration::from_secs(30))? {
        Ok(items) => Ok(items),
        Err(err) => Err(steam_err("Listing Workshop items failed", err)),
    }
}

pub fn publish(
    project: &AlbumProject,
    dest_dir: &str,
    options: &WorkshopPublishOptions,
    mut progress: impl FnMut(WorkshopProgress),
) -> AppResult<WorkshopPublishResult> {
    let client = connect()?;
    let title = truncate(&project.resolved_title(), TITLE_MAX);
    if title.trim().is_empty() {
        return Err(AppError::Message("Addon title is required to publish.".into()));
    }

    let update_id = options.workshop_id.or(project.workshop_id);
    let is_update = update_id.is_some();

    progress(stage("export", "Packing the album addon.", 8));
    let exported = export_for_workshop(project, dest_dir, |p| {
        let percent = 8 + (p.percent as u16 * 40 / 100) as u8;
        progress(stage("export", &p.detail, percent));
    })?;

    let gma_path = exported
        .gma_path
        .as_ref()
        .ok_or_else(|| AppError::Message("Export did not produce a .gma.".into()))?;
    let icon_path = exported
        .workshop_icon_path
        .as_ref()
        .ok_or_else(|| AppError::Message("Export did not produce a workshop icon.".into()))?;

    let staging = stage_content(gma_path)?;
    let description = resolved_description(project, options);
    let visibility = parse_visibility(&options.visibility);
    let tags = workshop_tags();
    let change_note = options.change_note.trim();
    let change_note = if change_note.is_empty() {
        None
    } else {
        Some(change_note.to_string())
    };

    progress(stage(
        "steam",
        if is_update {
            "Updating Workshop item."
        } else {
            "Creating Workshop item."
        },
        52,
    ));

    let (id, created) = match update_id {
        Some(id) => (PublishedFileId(id), false),
        None => {
            let created_id = create_item(&client)?;
            (created_id, true)
        }
    };

    let upload = upload_item(
        &client,
        id,
        staging.content_dir.as_path(),
        Path::new(icon_path),
        &title,
        &description,
        &tags,
        visibility,
        created,
        change_note.as_deref(),
        &mut progress,
    );

    let _ = std::fs::remove_dir_all(&staging.root);

    match upload {
        Ok(needs_legal_agreement) => {
            progress(stage(
                "steam",
                "Requiring Working Record Player on the Workshop.",
                96,
            ));
            let dependency_error = add_record_player_dependency(id)
                .err()
                .map(|err| err.to_string());
            Ok(WorkshopPublishResult {
                workshop_id: id.0,
                url: workshop_url(id.0),
                updated: is_update,
                needs_legal_agreement,
                legal_agreement_url: LEGAL_AGREEMENT_URL.to_string(),
                export: exported,
                dependency_error,
            })
        }
        Err(err) => {
            if created {
                client.ugc().delete_item(id, |_| {});
            }
            Err(err)
        }
    }
}

pub fn workshop_url(id: u64) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}")
}

pub fn default_description(project: &AlbumProject) -> String {
    standard_workshop_description(&project.artist, &project.album)
}

fn connect() -> AppResult<Client> {
    let mut slot = STEAM
        .lock()
        .map_err(|_| AppError::Message("Steam session lock was poisoned.".into()))?;
    if let Some(client) = slot.as_ref() {
        return Ok(client.clone());
    }

    let client = Client::init_app(GMOD_APP_ID).map_err(|err| {
        AppError::Message(format!(
            "Could not connect to Steam as Garry's Mod. Start Steam, log in, and make sure you own the game.\n{err}"
        ))
    })?;

    let runner = client.clone();
    CALLBACKS.get_or_init(|| {
        let _ = std::thread::Builder::new()
            .name("steam-callbacks".into())
            .spawn(move || loop {
                runner.run_callbacks();
                std::thread::sleep(Duration::from_millis(50));
            });
    });

    *slot = Some(client.clone());
    Ok(client)
}

fn export_for_workshop(
    project: &AlbumProject,
    dest_dir: &str,
    progress: impl FnMut(ExportProgress),
) -> AppResult<ExportResult> {
    let dest = if dest_dir.trim().is_empty() {
        std::env::temp_dir()
            .join("gmod-record-press")
            .to_string_lossy()
            .to_string()
    } else {
        dest_dir.to_string()
    };
    export_album(
        project,
        &ExportOptions {
            dest_dir: dest,
            write_gma: true,
            write_workshop_icon: true,
        },
        progress,
    )
}

struct Staging {
    root: PathBuf,
    content_dir: PathBuf,
}

fn stage_content(gma_path: &str) -> AppResult<Staging> {
    let src = Path::new(gma_path);
    if !src.is_file() {
        return Err(AppError::Message(format!("Packed .gma was not found:\n{gma_path}")));
    }
    let root = std::env::temp_dir().join(format!(
        "gmod-record-press-publish-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    let content_dir = root.join("content");
    std::fs::create_dir_all(&content_dir)?;
    std::fs::copy(src, content_dir.join("addon.gma"))?;
    Ok(Staging { root, content_dir })
}

fn create_item(client: &Client) -> AppResult<PublishedFileId> {
    let (tx, rx) = mpsc::channel();
    client
        .ugc()
        .create_item(GMOD_APP_ID, FileType::Community, move |result| {
            let _ = tx.send(result);
        });
    match recv_timeout(&rx, Duration::from_secs(60))? {
        Ok((id, _)) => Ok(id),
        Err(err) => Err(steam_err("Creating the Workshop item failed", err)),
    }
}

fn upload_item(
    client: &Client,
    id: PublishedFileId,
    content_dir: &Path,
    icon_path: &Path,
    title: &str,
    description: &str,
    tags: &[String],
    visibility: PublishedFileVisibility,
    is_create: bool,
    change_note: Option<&str>,
    progress: &mut impl FnMut(WorkshopProgress),
) -> AppResult<bool> {
    let (tx, rx) = mpsc::channel();
    let mut update = client
        .ugc()
        .start_item_update(GMOD_APP_ID, id)
        .content_path(content_dir)
        .preview_path(icon_path)
        .tags(tags.to_vec(), false)
        .visibility(visibility);

    if is_create || !title.trim().is_empty() {
        update = update.title(title);
    }
    if is_create || !description.trim().is_empty() {
        update = update.description(description);
    }

    let watch = update.submit(change_note, move |result| {
        let _ = tx.send(result);
    });

    loop {
        let (status, processed, total) = watch.progress();
        if !matches!(status, UpdateStatus::Invalid) {
            let detail = match status {
                UpdateStatus::PreparingConfig => "Preparing Workshop config.",
                UpdateStatus::PreparingContent => "Preparing addon content.",
                UpdateStatus::UploadingContent => "Uploading addon content.",
                UpdateStatus::UploadingPreviewFile => "Uploading workshop icon.",
                UpdateStatus::CommittingChanges => "Committing Workshop changes.",
                UpdateStatus::Invalid => unreachable!(),
            };
            let percent = if total > 0 {
                55 + ((processed.min(total) * 40) / total) as u8
            } else {
                match status {
                    UpdateStatus::PreparingConfig => 55,
                    UpdateStatus::PreparingContent => 62,
                    UpdateStatus::UploadingContent => 78,
                    UpdateStatus::UploadingPreviewFile => 88,
                    UpdateStatus::CommittingChanges => 94,
                    UpdateStatus::Invalid => 55,
                }
            };
            progress(stage("upload", detail, percent.min(99)));
        }

        match rx.try_recv() {
            Ok(Ok((_, legal))) => {
                progress(stage("done", "Workshop upload finished.", 100));
                return Ok(legal);
            }
            Ok(Err(err)) => return Err(steam_err("Workshop upload failed", err)),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(AppError::Message(
                    "Steam dropped the upload callback.".into(),
                ));
            }
            Err(mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(80));
            }
        }
    }
}

fn resolved_description(project: &AlbumProject, options: &WorkshopPublishOptions) -> String {
    if options.use_template {
        return truncate(&default_description(project), DESCRIPTION_MAX);
    }
    let explicit = options.description.trim();
    let from_project = project.workshop_description.trim();
    let text = if !explicit.is_empty() {
        explicit
    } else if !from_project.is_empty() {
        from_project
    } else {
        return truncate(&default_description(project), DESCRIPTION_MAX);
    };
    truncate(text, DESCRIPTION_MAX)
}

fn parse_visibility(value: &str) -> PublishedFileVisibility {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => PublishedFileVisibility::Public,
        "friends" | "friendsonly" | "friends_only" => PublishedFileVisibility::FriendsOnly,
        _ => PublishedFileVisibility::Private,
    }
}

fn workshop_tags() -> Vec<String> {
    vec![
        "Addon".into(),
        "entity".into(),
        "fun".into(),
        "roleplay".into(),
    ]
}

fn add_record_player_dependency(parent: PublishedFileId) -> AppResult<()> {
    let child = PublishedFileId(RECORD_PLAYER_WORKSHOP_ID);
    unsafe {
        let ugc = sys::SteamAPI_SteamUGC_v021();
        let utils = sys::SteamAPI_SteamUtils_v010();
        if ugc.is_null() || utils.is_null() {
            return Err(AppError::Message(
                "Steam UGC is not available to set the Working Record Player requirement.".into(),
            ));
        }

        let api_call = sys::SteamAPI_ISteamUGC_AddDependency(ugc, parent.0, child.0);
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let mut failed = false;
            if sys::SteamAPI_ISteamUtils_IsAPICallCompleted(utils, api_call, &mut failed) {
                let mut result = std::mem::MaybeUninit::<sys::AddUGCDependencyResult_t>::zeroed();
                let mut read_failed = false;
                let got = sys::SteamAPI_ISteamUtils_GetAPICallResult(
                    utils,
                    api_call,
                    result.as_mut_ptr().cast(),
                    std::mem::size_of::<sys::AddUGCDependencyResult_t>() as i32,
                    sys::AddUGCDependencyResult_t_k_iCallback as i32,
                    &mut read_failed,
                );
                if got {
                    let result = result.assume_init();
                    if !dependency_result_ok(result.m_eResult) {
                        return Err(steam_err(
                            "Could not require Working Record Player",
                            SteamError::from(result.m_eResult),
                        ));
                    }
                    return Ok(());
                }
                // The callback thread may already have consumed the result.
                if failed && read_failed {
                    return Err(AppError::Message(
                        "Steam could not set Working Record Player as a required item.".into(),
                    ));
                }
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(AppError::Message(
                    "Timed out setting Working Record Player as a required item.".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn dependency_result_ok(result: sys::EResult) -> bool {
    matches!(
        result,
        sys::EResult::k_EResultOK
            | sys::EResult::k_EResultDuplicateRequest
            | sys::EResult::k_EResultAlreadyOwned
            | sys::EResult::k_EResultDuplicateName
            | sys::EResult::k_EResultSameAsPreviousValue
    )
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    text.chars().take(max).collect()
}

fn stage(stage: &str, detail: &str, percent: u8) -> WorkshopProgress {
    WorkshopProgress {
        stage: stage.into(),
        detail: detail.into(),
        percent,
    }
}

fn steam_err(prefix: &str, err: SteamError) -> AppError {
    AppError::Message(format!("{prefix}: {err}"))
}

fn recv_timeout<T>(rx: &mpsc::Receiver<T>, timeout: Duration) -> AppResult<T> {
    let deadline = Instant::now() + timeout;
    loop {
        match rx.try_recv() {
            Ok(value) => return Ok(value),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(AppError::Message("Steam callback dropped.".into()));
            }
            Err(mpsc::TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    return Err(AppError::Message(
                        "Steam timed out. Is the Steam client running?".into(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;

    fn project() -> AlbumProject {
        AlbumProject {
            artist: "Test Artist".into(),
            album: "Demo Days".into(),
            vinyl_id: "demo_days".into(),
            addon_title: String::new(),
            cover_path: None,
            back_cover_path: None,
            label_path: None,
            vinyl_color: "#101010".into(),
            vinyl_resolution: 1024,
            tracks: vec![Track {
                name: "First Song".into(),
                path: "x.mp3".into(),
            }],
            workshop_id: None,
            workshop_description: String::new(),
            workshop_visibility: "private".into(),
            workshop_use_template: true,
        }
    }

    #[test]
    fn description_uses_copyright_template() {
        let text = default_description(&project());
        assert!(text.contains("[h1]Working Record Player - Test Artist - Demo Days[/h1]"));
        assert!(text.contains("[b]Working Record Player[/b]"));
        assert!(text.contains("Copyright Notice"));
        assert!(text.contains(
            "Created with [url=https://github.com/Aspenini/gmod-record-press]GMod Record Press[/url]."
        ));
        assert!(!text.contains("1. First Song"));
    }

    #[test]
    fn visibility_defaults_private() {
        assert!(matches!(
            parse_visibility("nope"),
            PublishedFileVisibility::Private
        ));
        assert!(matches!(
            parse_visibility("public"),
            PublishedFileVisibility::Public
        ));
        assert!(matches!(
            parse_visibility("friends"),
            PublishedFileVisibility::FriendsOnly
        ));
    }

    #[test]
    fn workshop_url_uses_id() {
        assert!(workshop_url(123).ends_with("id=123"));
    }

    #[test]
    fn record_player_requirement_id() {
        assert_eq!(RECORD_PLAYER_WORKSHOP_ID, 3777821069);
        assert!(workshop_url(RECORD_PLAYER_WORKSHOP_ID).ends_with("id=3777821069"));
    }

    #[test]
    fn already_required_is_not_an_error() {
        assert!(dependency_result_ok(sys::EResult::k_EResultOK));
        assert!(dependency_result_ok(sys::EResult::k_EResultDuplicateRequest));
        assert!(!dependency_result_ok(sys::EResult::k_EResultFail));
        assert!(!dependency_result_ok(sys::EResult::k_EResultAccessDenied));
    }

    #[test]
    fn title_follows_working_record_player_scheme() {
        assert_eq!(
            crate::model::standard_addon_title("Hank Williams Jr.", "If The South Woulda Won"),
            "[Working Record Player] Hank Williams Jr. - If The South Woulda Won"
        );
    }
}
