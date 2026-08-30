use crate::error::{AppError, AppResult};
use crate::gma::{write_gma, GmaFile};
use crate::lua::{addon_json, case_vmt, render_autorun, vinyl_vmt};
use crate::model::{has_errors, validate_project, AlbumProject, ExportOptions, ExportProgress, ExportResult};
use crate::slug::{sanitize_filename_stem, unique_filename};
use crate::vinyl_art::render_vinyl;
use crate::vtf_encode::{
    cover_square, encode_dxt1_vtf, encode_png, encode_workshop_jpeg, fit_max_edge, load_image,
};
use std::fs;
use std::path::{Path, PathBuf};

pub fn export_album(
    project: &AlbumProject,
    options: &ExportOptions,
    mut progress: impl FnMut(ExportProgress),
) -> AppResult<ExportResult> {
    let issues = validate_project(project);
    if has_errors(&issues) {
        let first = issues
            .iter()
            .find(|i| i.level == "error")
            .map(|i| i.message.clone())
            .unwrap_or_else(|| "Project is not ready to export.".into());
        return Err(AppError::Message(first));
    }

    progress(stage("validate", "Project looks good.", 4));

    let dest_parent = PathBuf::from(&options.dest_dir);
    if dest_parent.as_os_str().is_empty() {
        return Err(AppError::Message("Choose an export folder.".into()));
    }
    fs::create_dir_all(&dest_parent)?;

    let addon_dir = dest_parent.join(project.addon_folder_name());
    let id = project.vinyl_id.trim();

    progress(stage("artwork", "Reading cover, back, and label.", 8));
    let cover_path = project
        .cover_path
        .as_ref()
        .ok_or_else(|| AppError::Message("Front cover is required.".into()))?;
    let cover = load_required_image(Path::new(cover_path), "front cover")?;
    let back_img = match project.back_cover_path.as_ref() {
        Some(p) if !p.trim().is_empty() => load_required_image(Path::new(p), "back cover")?,
        _ => cover.clone(),
    };
    let label_img = match project.label_path.as_ref() {
        Some(p) if !p.trim().is_empty() => load_required_image(Path::new(p), "vinyl label")?,
        _ => cover.clone(),
    };

    let mut used_names = Vec::new();
    let mut track_pairs = Vec::new();
    for track in &project.tracks {
        let src = Path::new(&track.path);
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp3")
            .to_ascii_lowercase();
        let stem = sanitize_filename_stem(
            src.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("track"),
        );
        let file_name = unique_filename(&stem, &ext, &mut used_names);
        track_pairs.push((track.name.trim().to_string(), file_name));
    }

    progress(stage("audio", "Snapshotting tracks.", 16));
    let staging = std::env::temp_dir().join(format!(
        "gmod-record-press-export-{}-{}",
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    fs::create_dir_all(&staging)?;
    let staged_tracks = snapshot_tracks(project, &track_pairs, &staging);

    let staged_tracks = match staged_tracks {
        Ok(tracks) => tracks,
        Err(err) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(err);
        }
    };

    if addon_dir.exists() {
        fs::remove_dir_all(&addon_dir)?;
    }

    let lua_dir = addon_dir.join("lua/autorun");
    let mat_dir = addon_dir.join("materials/recordplayer").join(id);
    let sound_dir = addon_dir.join("sound/recordplayer").join(id);
    fs::create_dir_all(&lua_dir)?;
    fs::create_dir_all(&mat_dir)?;
    fs::create_dir_all(&sound_dir)?;

    progress(stage("setup", "Writing addon.json and Lua.", 22));

    let title = project.resolved_title();
    fs::write(
        addon_dir.join("addon.json"),
        serde_json::to_string_pretty(&addon_json(
            &title,
            project.workshop_id,
            &project.vinyl_color,
            project.vinyl_resolution,
        ))?,
    )?;

    let lua_tracks: Vec<(String, String)> = track_pairs
        .iter()
        .map(|(name, file)| (name.clone(), format!("recordplayer/{id}/{file}")))
        .collect();
    fs::write(
        lua_dir.join(format!("recordplayer-{id}.lua")),
        render_autorun(project, &lua_tracks),
    )?;

    progress(stage("artwork", "Preparing cover and case textures.", 32));

    let cover_png = encode_png(&fit_max_edge(&cover, 1024))?;
    fs::write(mat_dir.join("cover.png"), &cover_png)?;

    let case_front = cover_square(&cover, 512);
    let case_front_vtf = encode_dxt1_vtf(&case_front)?;
    fs::write(mat_dir.join("case_front.vtf"), &case_front_vtf)?;
    fs::write(mat_dir.join("case_front.vmt"), case_vmt(id, "case_front"))?;

    fs::write(
        mat_dir.join("back.png"),
        encode_png(&fit_max_edge(&back_img, 1024))?,
    )?;
    let case_back = cover_square(&back_img, 512);
    let case_back_vtf = encode_dxt1_vtf(&case_back)?;
    fs::write(mat_dir.join("case_back.vtf"), &case_back_vtf)?;
    fs::write(mat_dir.join("case_back.vmt"), case_vmt(id, "case_back"))?;

    progress(stage("vinyl", "Pressing the vinyl texture.", 52));

    fs::write(
        mat_dir.join("label.png"),
        encode_png(&fit_max_edge(&label_img, 1024))?,
    )?;
    let vinyl = render_vinyl(&label_img, &project.vinyl_color, project.vinyl_resolution);
    let vinyl_vtf = encode_dxt1_vtf(&vinyl)?;
    fs::write(mat_dir.join("vinyl.vtf"), &vinyl_vtf)?;
    fs::write(mat_dir.join("vinyl.vmt"), vinyl_vmt(id))?;

    progress(stage("audio", "Copying tracks.", 72));

    for (src, file_name) in staged_tracks {
        fs::copy(&src, sound_dir.join(file_name))?;
    }
    let _ = fs::remove_dir_all(&staging);

    let mut files_written = 12 + project.tracks.len();

    let workshop_icon_path = if options.write_workshop_icon {
        progress(stage("icon", "Writing workshop icon.", 84));
        let icon_path = dest_parent.join(format!("{}.jpg", project.addon_folder_name()));
        fs::write(&icon_path, encode_workshop_jpeg(&cover)?)?;
        files_written += 1;
        Some(icon_path.to_string_lossy().to_string())
    } else {
        None
    };

    let gma_path = if options.write_gma {
        progress(stage("gma", "Packing .gma.", 90));
        let packed = collect_gma_files(&addon_dir)?;
        let gma = dest_parent.join(format!("{}.gma", project.addon_folder_name()));
        write_gma(&gma, &title, &packed)?;
        files_written += 1;
        Some(gma.to_string_lossy().to_string())
    } else {
        None
    };

    progress(stage("done", "Album addon is ready.", 100));

    Ok(ExportResult {
        addon_dir: addon_dir.to_string_lossy().to_string(),
        gma_path,
        workshop_icon_path,
        files_written,
    })
}

fn load_required_image(path: &Path, what: &str) -> AppResult<image::DynamicImage> {
    load_image(path).map_err(|err| {
        AppError::Message(format!(
            "Could not read {what}:\n{}\n{err}",
            path.display()
        ))
    })
}

fn snapshot_tracks(
    project: &AlbumProject,
    track_pairs: &[(String, String)],
    staging: &Path,
) -> AppResult<Vec<(PathBuf, String)>> {
    let mut staged = Vec::new();
    for (track, (_, file_name)) in project.tracks.iter().zip(track_pairs.iter()) {
        let src = Path::new(&track.path);
        if !src.is_file() {
            return Err(AppError::Message(format!(
                "Track \"{}\" audio was not found:\n{}",
                track.name.trim(),
                track.path
            )));
        }
        let dest = staging.join(file_name);
        fs::copy(src, &dest).map_err(|err| {
            AppError::Message(format!(
                "Could not snapshot track \"{}\":\n{}\n{err}",
                track.name.trim(),
                track.path
            ))
        })?;
        staged.push((dest, file_name.clone()));
    }
    Ok(staged)
}

fn collect_gma_files(root: &Path) -> AppResult<Vec<GmaFile>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.push(GmaFile {
            path: rel,
            data: fs::read(entry.path())?,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn stage(stage: &str, detail: &str, percent: u8) -> ExportProgress {
    ExportProgress {
        stage: stage.into(),
        detail: detail.into(),
        percent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn export_writes_expected_tree() {
        let dir = tempdir().unwrap();
        let cover = dir.path().join("cover.png");
        RgbaImage::from_pixel(64, 64, Rgba([30, 80, 180, 255]))
            .save(&cover)
            .unwrap();
        let audio = dir.path().join("song.mp3");
        fs::write(&audio, b"ID3fake").unwrap();

        let project = AlbumProject {
            artist: "Test Artist".into(),
            album: "Demo Days".into(),
            vinyl_id: "demo_days".into(),
            addon_title: String::new(),
            cover_path: Some(cover.to_string_lossy().to_string()),
            back_cover_path: None,
            label_path: None,
            vinyl_color: "#101010".into(),
            vinyl_resolution: 1024,
            tracks: vec![Track {
                name: "First Song".into(),
                path: audio.to_string_lossy().to_string(),
            }],
            workshop_id: None,
            workshop_description: String::new(),
            workshop_visibility: "private".into(),
            workshop_use_template: true,
        };
        let dest = dir.path().join("out");
        let result = export_album(
            &project,
            &ExportOptions {
                dest_dir: dest.to_string_lossy().to_string(),
                write_gma: true,
                write_workshop_icon: true,
            },
            |_| {},
        )
        .unwrap();

        let addon = PathBuf::from(&result.addon_dir);
        assert!(addon.join("addon.json").is_file());
        assert!(addon.join("lua/autorun/recordplayer-demo_days.lua").is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/case_front.vtf")
            .is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/vinyl.vtf")
            .is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/back.png")
            .is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/label.png")
            .is_file());
        assert!(addon
            .join("sound/recordplayer/demo_days/song.mp3")
            .is_file());
        assert!(result.gma_path.is_some());
        assert!(result.workshop_icon_path.is_some());

        let lua = fs::read_to_string(addon.join("lua/autorun/recordplayer-demo_days.lua")).unwrap();
        assert!(lua.contains("RegisterVinyl(\"demo_days\""));
        assert!(lua.contains("First Song"));
    }

    #[test]
    fn reexport_over_existing_addon_does_not_eat_sources() {
        let dir = tempdir().unwrap();
        let cover = dir.path().join("cover.png");
        RgbaImage::from_pixel(64, 64, Rgba([30, 80, 180, 255]))
            .save(&cover)
            .unwrap();
        let audio = dir.path().join("song.mp3");
        fs::write(&audio, b"ID3fake").unwrap();

        let mut project = AlbumProject {
            artist: "Test Artist".into(),
            album: "Demo Days".into(),
            vinyl_id: "demo_days".into(),
            addon_title: String::new(),
            cover_path: Some(cover.to_string_lossy().to_string()),
            back_cover_path: None,
            label_path: None,
            vinyl_color: "#101010".into(),
            vinyl_resolution: 1024,
            tracks: vec![Track {
                name: "First Song".into(),
                path: audio.to_string_lossy().to_string(),
            }],
            workshop_id: None,
            workshop_description: String::new(),
            workshop_visibility: "private".into(),
            workshop_use_template: true,
        };
        let dest = dir.path().join("out");
        let options = ExportOptions {
            dest_dir: dest.to_string_lossy().to_string(),
            write_gma: false,
            write_workshop_icon: false,
        };
        let first = export_album(&project, &options, |_| {}).unwrap();
        let addon = PathBuf::from(&first.addon_dir);
        let exported_cover = addon.join("materials/recordplayer/demo_days/cover.png");
        let exported_audio = addon.join("sound/recordplayer/demo_days/song.mp3");
        assert!(exported_cover.is_file());
        assert!(exported_audio.is_file());

        project.cover_path = Some(exported_cover.to_string_lossy().to_string());
        project.tracks[0].path = exported_audio.to_string_lossy().to_string();

        let second = export_album(&project, &options, |_| {}).unwrap();
        let addon = PathBuf::from(&second.addon_dir);
        assert!(addon
            .join("materials/recordplayer/demo_days/cover.png")
            .is_file());
        assert!(addon
            .join("sound/recordplayer/demo_days/song.mp3")
            .is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/back.png")
            .is_file());
        assert!(addon
            .join("materials/recordplayer/demo_days/label.png")
            .is_file());
        let copied = fs::read(addon.join("sound/recordplayer/demo_days/song.mp3")).unwrap();
        assert_eq!(copied, b"ID3fake");
    }
}
