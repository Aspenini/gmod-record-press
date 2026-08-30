import { invoke } from "@tauri-apps/api/core";
import type {
  AlbumProject,
  AudioInfo,
  ExportOptions,
  ExportResult,
  ImagePreview,
  Issue,
} from "../types";

export const api = {
  slugifyId: (input: string) => invoke<string>("slugify_id", { input }),
  validate: (project: AlbumProject) => invoke<Issue[]>("validate", { project }),
  suggestGmodAddonsDir: () => invoke<string | null>("suggest_gmod_addons_dir"),
  audioInfo: (paths: string[]) => invoke<AudioInfo[]>("audio_info", { paths }),
  readImagePreview: (path: string) =>
    invoke<ImagePreview>("read_image_preview", { path }),
  pickImage: () => invoke<ImagePreview | null>("pick_image"),
  pickAudioFiles: () => invoke<AudioInfo[]>("pick_audio_files"),
  pickExportDir: () => invoke<string | null>("pick_export_dir"),
  pickSaveProject: () => invoke<string | null>("pick_save_project"),
  pickOpenProject: () => invoke<string | null>("pick_open_project"),
  saveProject: (path: string, project: AlbumProject) =>
    invoke<void>("save_project", { path, project }),
  loadProject: (path: string) => invoke<AlbumProject>("load_project", { path }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  exportAddon: (project: AlbumProject, options: ExportOptions) =>
    invoke<ExportResult>("export_addon", { project, options }),
};
