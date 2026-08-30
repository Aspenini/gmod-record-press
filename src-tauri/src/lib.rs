mod audio_meta;
mod error;
mod export;
mod gma;
mod import;
mod lua;
mod model;
mod slug;
mod steam;
mod vinyl_art;
mod vtf_encode;
mod workshop;

use crate::error::AppResult;
use crate::model::{
    validate_project, AlbumProject, AudioScan, ExportOptions, ExportProgress, ExportResult,
    ImagePreview, Issue, VinylLibrary, WorkshopItem, WorkshopPublishOptions, WorkshopPublishResult,
    WorkshopProgress, WorkshopStatus,
};
use crate::vtf_encode::{load_image, preview_data_url};
use std::path::Path;
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn slugify_id(input: String) -> String {
    slug::slugify(&input)
}

#[tauri::command]
fn validate(project: AlbumProject) -> Vec<Issue> {
    validate_project(&project)
}

#[tauri::command]
async fn suggest_gmod_addons_dir() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(steam::suggest_gmod_addons_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn audio_info(paths: Vec<String>) -> Result<AudioScan, String> {
    tauri::async_runtime::spawn_blocking(move || audio_meta::scan_audio(&paths))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_image_preview(path: String) -> Result<ImagePreview, String> {
    tauri::async_runtime::spawn_blocking(move || read_preview(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(Into::into)
}

#[tauri::command]
async fn pick_image(app: tauri::AppHandle) -> Result<Option<ImagePreview>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "tga"])
            .set_title("Choose artwork")
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    match picked.and_then(|p| p.into_path().ok()) {
        Some(path) => Ok(Some(read_preview(&path).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

#[tauri::command]
async fn pick_audio_files(app: tauri::AppHandle) -> Result<AudioScan, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Audio", &["mp3", "ogg", "wav"])
            .set_title("Choose tracks")
            .blocking_pick_files()
    })
    .await
    .map_err(|e| e.to_string())?;

    let Some(files) = picked else {
        return Ok(AudioScan::default());
    };

    let paths: Vec<String> = files
        .into_iter()
        .filter_map(|f| f.into_path().ok())
        .map(|path| path.to_string_lossy().to_string())
        .collect();
    Ok(audio_meta::scan_audio(&paths))
}

#[tauri::command]
async fn pick_export_dir(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose export folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(picked.and_then(|p| p.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
async fn pick_save_project(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Album project", &["json"])
            .set_file_name("album.json")
            .set_title("Save album project")
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(picked.and_then(|p| p.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
async fn pick_open_project(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Album project", &["json"])
            .set_title("Open album project")
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(picked.and_then(|p| p.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
fn save_project(path: String, project: AlbumProject) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_project(path: String) -> Result<AlbumProject, String> {
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_vinyl_addons(scan_dir: Option<String>) -> Result<VinylLibrary, String> {
    tauri::async_runtime::spawn_blocking(move || import::list_vinyl_library(scan_dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_vinyl_addon(path: String) -> Result<AlbumProject, String> {
    tauri::async_runtime::spawn_blocking(move || import::import_vinyl_addon(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(Into::into)
}

#[tauri::command]
async fn pick_addon_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose a vinyl addon folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(picked.and_then(|p| p.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn export_addon(
    app: tauri::AppHandle,
    project: AlbumProject,
    options: ExportOptions,
) -> Result<ExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        export::export_album(&project, &options, |progress: ExportProgress| {
            let _ = app.emit("export-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(Into::into)
}

#[tauri::command]
async fn steam_status() -> Result<WorkshopStatus, String> {
    tauri::async_runtime::spawn_blocking(workshop::status)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_workshop_items() -> Result<Vec<WorkshopItem>, String> {
    tauri::async_runtime::spawn_blocking(workshop::list_my_items)
        .await
        .map_err(|e| e.to_string())?
        .map_err(Into::into)
}

#[tauri::command]
fn workshop_description(project: AlbumProject) -> String {
    workshop::default_description(&project)
}

#[tauri::command]
async fn publish_workshop(
    app: tauri::AppHandle,
    project: AlbumProject,
    options: WorkshopPublishOptions,
) -> Result<WorkshopPublishResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        workshop::publish(&project, &options.dest_dir, &options, |progress: WorkshopProgress| {
            let _ = app.emit("workshop-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(Into::into)
}

fn read_preview(path: &Path) -> AppResult<ImagePreview> {
    let img = load_image(path)?;
    let (width, height) = image::GenericImageView::dimensions(&img);
    Ok(ImagePreview {
        path: path.to_string_lossy().to_string(),
        data_url: preview_data_url(&img, 720)?,
        width,
        height,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            slugify_id,
            validate,
            suggest_gmod_addons_dir,
            audio_info,
            read_image_preview,
            pick_image,
            pick_audio_files,
            pick_export_dir,
            pick_save_project,
            pick_open_project,
            pick_addon_folder,
            save_project,
            load_project,
            list_vinyl_addons,
            import_vinyl_addon,
            open_path,
            export_addon,
            steam_status,
            list_workshop_items,
            workshop_description,
            publish_workshop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
