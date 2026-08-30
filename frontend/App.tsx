import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import packageInfo from "../package.json";
import { Dropzone } from "./components/Dropzone";
import { MetadataPicker } from "./components/MetadataPicker";
import { OpenPanel } from "./components/OpenPanel";
import { Preview } from "./components/Preview";
import { TrackList } from "./components/TrackList";
import { ViewSwitch } from "./components/ViewSwitch";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./lib/api";
import { suggestAlbumMeta, type AlbumMetaSuggestion } from "./lib/audioMeta";
import {
  defaultAddonTitle,
  defaultWorkshopDescription,
  parseWorkshopId,
  slugify,
  WORKSHOP_TITLE_PREFIX,
} from "./lib/slug";
import type {
  AlbumProject,
  AudioInfo,
  AudioScan,
  ExportProgress,
  ExportResult,
  ImagePreview,
  Issue,
  Track,
  VinylAddonInfo,
  WorkshopItem,
  WorkshopPublishResult,
  WorkshopStatus,
} from "./types";
import { RECORD_PLAYER_WORKSHOP_URL, VINYL_COLORS } from "./types";

const IMAGE_EXT = [".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tga"];
const AUDIO_EXT = [".mp3", ".ogg", ".wav"];

type DropTarget = "cover" | "back" | "label" | "tracks" | null;
type LeftView = "album" | "preview";
type RightView = "export" | "workshop";

function newTrack(info: AudioInfo): Track {
  return {
    id: crypto.randomUUID(),
    name: info.suggestedName,
    path: info.path,
    fileName: info.fileName,
    size: info.size,
  };
}

export default function App() {
  const [artist, setArtist] = useState("");
  const [album, setAlbum] = useState("");
  const [vinylId, setVinylId] = useState("");
  const [idTouched, setIdTouched] = useState(false);
  const [addonTitle, setAddonTitle] = useState("[Working Record Player]");
  const [cover, setCover] = useState<ImagePreview | null>(null);
  const [back, setBack] = useState<ImagePreview | null>(null);
  const [label, setLabel] = useState<ImagePreview | null>(null);
  const [vinylColor, setVinylColor] = useState("#141414");
  const [vinylResolution, setVinylResolution] = useState(2048);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [destDir, setDestDir] = useState("");
  const [gmodDir, setGmodDir] = useState<string | null>(null);
  const [writeGma, setWriteGma] = useState(false);
  const [writeIcon, setWriteIcon] = useState(true);
  const [workshopId, setWorkshopId] = useState("");
  const [workshopDescription, setWorkshopDescription] = useState("");
  const [useDescriptionTemplate, setUseDescriptionTemplate] = useState(true);
  const [workshopVisibility, setWorkshopVisibility] = useState("private");
  const [changeNote, setChangeNote] = useState("");
  const [steam, setSteam] = useState<WorkshopStatus | null>(null);
  const [workshopItems, setWorkshopItems] = useState<WorkshopItem[]>([]);
  const [showOnlyRelevantWorkshopItems, setShowOnlyRelevantWorkshopItems] = useState(true);
  const [issues, setIssues] = useState<Issue[]>([]);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [workshopResult, setWorkshopResult] = useState<WorkshopPublishResult | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget>(null);
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [openPanel, setOpenPanel] = useState(false);
  const [openLoading, setOpenLoading] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);
  const [libraryAddons, setLibraryAddons] = useState<VinylAddonInfo[]>([]);
  const [libraryDir, setLibraryDir] = useState<string | null>(null);
  const [leftView, setLeftView] = useState<LeftView>("album");
  const [rightView, setRightView] = useState<RightView>("export");
  const [metaPicker, setMetaPicker] = useState<AlbumMetaSuggestion | null>(null);
  const metaRef = useRef({
    artist: "",
    album: "",
    cover: null as ImagePreview | null,
    back: null as ImagePreview | null,
    label: null as ImagePreview | null,
    tracks: [] as Track[],
  });
  metaRef.current = { artist, album, cover, back, label, tracks };
  const dropHandlerRef = useRef<(paths: string[], target: DropTarget) => void>(() => {});

  const project = useMemo<AlbumProject>(
    () => ({
      artist,
      album,
      vinylId,
      addonTitle,
      coverPath: cover?.path ?? null,
      backCoverPath: back?.path ?? null,
      labelPath: label?.path ?? null,
      vinylColor,
      vinylResolution,
      tracks: tracks.map(({ name, path }) => ({ name, path })),
      workshopId: parseWorkshopId(workshopId),
      workshopDescription,
      workshopVisibility,
      workshopUseTemplate: useDescriptionTemplate,
    }),
    [
      artist,
      album,
      vinylId,
      addonTitle,
      cover,
      back,
      label,
      vinylColor,
      vinylResolution,
      tracks,
      workshopId,
      workshopDescription,
      workshopVisibility,
      useDescriptionTemplate,
    ],
  );

  const visibleWorkshopItems = useMemo(
    () =>
      showOnlyRelevantWorkshopItems
        ? workshopItems.filter((item) => item.title.startsWith(WORKSHOP_TITLE_PREFIX))
        : workshopItems,
    [showOnlyRelevantWorkshopItems, workshopItems],
  );

  useEffect(() => {
    api.suggestGmodAddonsDir().then((dir) => {
      setGmodDir(dir);
      const last = localStorage.getItem("rpam.destDir");
      if (last) setDestDir(last);
      else if (dir) setDestDir(dir);
    });
    refreshSteam();
  }, []);

  useEffect(() => {
    if (!idTouched) setVinylId(album.trim() ? slugify(album) : "");
  }, [album, idTouched]);

  useEffect(() => {
    setAddonTitle(defaultAddonTitle(artist, album));
  }, [artist, album]);

  useEffect(() => {
    if (!useDescriptionTemplate) return;
    setWorkshopDescription(defaultWorkshopDescription(artist, album));
  }, [artist, album, useDescriptionTemplate]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      api.validate(project).then(setIssues).catch(() => setIssues([]));
    }, 200);
    return () => window.clearTimeout(handle);
  }, [project]);

  useEffect(() => {
    let cancelled = false;
    let unlistenDrop: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenWorkshop: (() => void) | undefined;

    listen<ExportProgress>("export-progress", (event) => {
      if (!cancelled) setProgress(event.payload);
    }).then((fn) => {
      unlistenProgress = fn;
    });

    listen<ExportProgress>("workshop-progress", (event) => {
      if (!cancelled) setProgress(event.payload);
    }).then((fn) => {
      unlistenWorkshop = fn;
    });

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "leave") {
          setDropTarget(null);
          return;
        }
        const scale = window.devicePixelRatio || 1;
        const pos =
          "position" in event.payload ? event.payload.position : null;
        const target = pos
          ? targetFromPoint(pos.x / scale, pos.y / scale)
          : "tracks";
        if (event.payload.type === "over" || event.payload.type === "enter") {
          setDropTarget(target);
        }
        if (event.payload.type === "drop") {
          setDropTarget(null);
          dropHandlerRef.current(event.payload.paths, target);
        }
      })
      .then((fn) => {
        unlistenDrop = fn;
      });

    return () => {
      cancelled = true;
      unlistenDrop?.();
      unlistenProgress?.();
      unlistenWorkshop?.();
    };
  }, []);

  async function handleDroppedPaths(paths: string[], target: DropTarget) {
    const images = paths.filter((p) =>
      IMAGE_EXT.some((ext) => p.toLowerCase().endsWith(ext)),
    );
    const audio = paths.filter((p) =>
      AUDIO_EXT.some((ext) => p.toLowerCase().endsWith(ext)),
    );

    if (audio.length) {
      await ingestAudio(await api.audioInfo(audio));
    }

    if (!images.length) return;
    const preview = await api.readImagePreview(images[0]);
    if (target === "back") setBack(preview);
    else if (target === "label") setLabel(preview);
    else setCover(preview);
  }
  dropHandlerRef.current = handleDroppedPaths;

  async function pickArt(which: "cover" | "back" | "label") {
    const picked = await api.pickImage();
    if (!picked) return;
    if (which === "cover") setCover(picked);
    if (which === "back") setBack(picked);
    if (which === "label") setLabel(picked);
  }

  async function addTracks() {
    const scan = await api.pickAudioFiles();
    if (!scan.tracks.length) return;
    await ingestAudio(scan);
  }

  async function ingestAudio(scan: AudioScan) {
    const known = new Set(metaRef.current.tracks.map((t) => t.path));
    const added = scan.tracks.filter((info) => !known.has(info.path));
    if (!added.length) return;
    setTracks((cur) => {
      const seen = new Set(cur.map((t) => t.path));
      return [...cur, ...added.filter((info) => !seen.has(info.path)).map(newTrack)];
    });

    const current = metaRef.current;
    const suggestion = suggestAlbumMeta(added, scan.pictures, {
      artist: current.artist,
      album: current.album,
      hasCover: Boolean(current.cover),
      hasBack: Boolean(current.back),
      hasLabel: Boolean(current.label),
    });
    applyAlbumMeta(suggestion);
    if (suggestion.textConflicts.length || suggestion.artConflicts.length) {
      setMetaPicker(suggestion);
    }
  }

  function applyAlbumMeta(picked: {
    artist?: string;
    album?: string;
    cover?: ImagePreview;
    back?: ImagePreview;
    label?: ImagePreview;
  }) {
    if (picked.artist) {
      setArtist((cur) => (cur.trim() ? cur : picked.artist!));
    }
    if (picked.album) {
      setAlbum((cur) => (cur.trim() ? cur : picked.album!));
    }
    if (picked.cover) {
      setCover((cur) => cur ?? picked.cover ?? null);
    }
    if (picked.back) {
      setBack((cur) => cur ?? picked.back ?? null);
    }
    if (picked.label) {
      setLabel((cur) => cur ?? picked.label ?? null);
    }
  }

  function moveTrack(id: string, dir: -1 | 1) {
    setTracks((cur) => {
      const index = cur.findIndex((t) => t.id === id);
      const next = index + dir;
      if (index < 0 || next < 0 || next >= cur.length) return cur;
      const copy = [...cur];
      [copy[index], copy[next]] = [copy[next], copy[index]];
      return copy;
    });
  }

  async function chooseDest() {
    const dir = await api.pickExportDir();
    if (dir) {
      setDestDir(dir);
      localStorage.setItem("rpam.destDir", dir);
    }
  }

  async function refreshSteam() {
    try {
      const status = await api.steamStatus();
      setSteam(status);
      if (status.connected) {
        const items = await api.listWorkshopItems().catch(() => []);
        setWorkshopItems(items);
      } else {
        setWorkshopItems([]);
      }
    } catch (err) {
      setSteam({
        connected: false,
        persona: null,
        error: err instanceof Error ? err.message : String(err),
      });
      setWorkshopItems([]);
    }
  }

  async function exportAlbum() {
    setError(null);
    setResult(null);
    setWorkshopResult(null);
    setBusy(true);
    setProgress({ stage: "start", detail: "Starting export…", percent: 1 });
    try {
      localStorage.setItem("rpam.destDir", destDir);
      const exported = await api.exportAddon(project, {
        destDir,
        writeGma,
        writeWorkshopIcon: writeIcon,
      });
      setResult(exported);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function publishToWorkshop() {
    setError(null);
    setResult(null);
    setWorkshopResult(null);
    setBusy(true);
    setProgress({ stage: "start", detail: "Connecting to Steam…", percent: 1 });
    try {
      if (destDir) localStorage.setItem("rpam.destDir", destDir);
      const published = await api.publishWorkshop(project, {
        destDir,
        workshopId: parseWorkshopId(workshopId),
        description: useDescriptionTemplate ? "" : workshopDescription,
        visibility: workshopVisibility,
        changeNote,
        useTemplate: useDescriptionTemplate,
      });
      setWorkshopResult(published);
      setResult(published.export);
      setWorkshopId(String(published.workshopId));
      await openUrl(published.url);
      if (published.needsLegalAgreement) {
        await openUrl(published.legalAgreementUrl);
      }
      void refreshSteam();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function saveProject() {
    const path = projectPath ?? (await api.pickSaveProject());
    if (!path) return;
    await api.saveProject(path, project);
    setProjectPath(path);
  }

  async function showOpenPanel() {
    setOpenPanel(true);
    setOpenError(null);
    setOpenLoading(true);
    try {
      const library = await api.listVinylAddons();
      setLibraryAddons(library.addons);
      setLibraryDir(library.scannedDir);
      if (library.gmodAddonsDir) setGmodDir(library.gmodAddonsDir);
    } catch (err) {
      setLibraryAddons([]);
      setOpenError(err instanceof Error ? err.message : String(err));
    } finally {
      setOpenLoading(false);
    }
  }

  async function applyLoadedProject(loaded: AlbumProject, jsonPath: string | null) {
    setArtist(loaded.artist);
    setAlbum(loaded.album);
    setVinylId(loaded.vinylId);
    setIdTouched(true);
    setAddonTitle(defaultAddonTitle(loaded.artist, loaded.album));
    setVinylColor(loaded.vinylColor);
    setVinylResolution(loaded.vinylResolution);
    setCover(
      loaded.coverPath
        ? await api.readImagePreview(loaded.coverPath).catch(() => null)
        : null,
    );
    setBack(
      loaded.backCoverPath
        ? await api.readImagePreview(loaded.backCoverPath).catch(() => null)
        : null,
    );
    setLabel(
      loaded.labelPath
        ? await api.readImagePreview(loaded.labelPath).catch(() => null)
        : null,
    );
    const infos = loaded.tracks.length
      ? (await api.audioInfo(loaded.tracks.map((t) => t.path))).tracks
      : [];
    const infoByPath = new Map(infos.map((info) => [info.path, info]));
    setTracks(
      loaded.tracks.map((track) => {
        const info = infoByPath.get(track.path);
        return {
          id: crypto.randomUUID(),
          name: track.name,
          path: track.path,
          fileName: info?.fileName ?? track.path.split(/[\\/]/).pop() ?? "track",
          size: info?.size ?? 0,
        };
      }),
    );
    setProjectPath(jsonPath);
    setResult(null);
    setWorkshopResult(null);
    setWorkshopId(loaded.workshopId ? String(loaded.workshopId) : "");
    const useTemplate = loaded.workshopUseTemplate !== false;
    setUseDescriptionTemplate(useTemplate);
    setWorkshopDescription(
      useTemplate
        ? defaultWorkshopDescription(loaded.artist, loaded.album)
        : loaded.workshopDescription ?? "",
    );
    setWorkshopVisibility(loaded.workshopVisibility || "private");
    setChangeNote("");
    setOpenPanel(false);
    setOpenError(null);
    setMetaPicker(null);
  }

  async function openVinylAddon(path: string) {
    setOpenError(null);
    try {
      const loaded = await api.importVinylAddon(path);
      const parent = path.replace(/[\\/]+$/, "").replace(/[\\/][^\\/]+$/, "");
      if (parent) {
        setDestDir(parent);
        localStorage.setItem("rpam.destDir", parent);
      }
      await applyLoadedProject(loaded, null);
    } catch (err) {
      setOpenError(err instanceof Error ? err.message : String(err));
    }
  }

  async function browseAddonFolder() {
    const dir = await api.pickAddonFolder();
    if (!dir) return;
    setOpenError(null);
    try {
      const loaded = await api.importVinylAddon(dir);
      const parent = dir.replace(/[\\/]+$/, "").replace(/[\\/][^\\/]+$/, "");
      if (parent) {
        setDestDir(parent);
        localStorage.setItem("rpam.destDir", parent);
      }
      await applyLoadedProject(loaded, null);
      return;
    } catch {
      // Not a single addon — maybe a folder of albums.
    }
    setOpenLoading(true);
    try {
      const library = await api.listVinylAddons(dir);
      setLibraryAddons(library.addons);
      setLibraryDir(library.scannedDir);
      if (!library.addons.length) {
        setOpenError("No record player albums in that folder.");
      }
    } catch (err) {
      setOpenError(err instanceof Error ? err.message : String(err));
    } finally {
      setOpenLoading(false);
    }
  }

  async function openProjectFile() {
    const path = await api.pickOpenProject();
    if (!path) return;
    try {
      const loaded = await api.loadProject(path);
      await applyLoadedProject(loaded, path);
    } catch (err) {
      setOpenError(err instanceof Error ? err.message : String(err));
    }
  }

  function reset() {
    setArtist("");
    setAlbum("");
    setVinylId("");
    setIdTouched(false);
    setAddonTitle("[Working Record Player]");
    setCover(null);
    setBack(null);
    setLabel(null);
    setVinylColor("#141414");
    setVinylResolution(2048);
    setTracks([]);
    setResult(null);
    setWorkshopResult(null);
    setError(null);
    setProjectPath(null);
    setWorkshopId("");
    setWorkshopDescription(defaultWorkshopDescription("", ""));
    setUseDescriptionTemplate(true);
    setWorkshopVisibility("private");
    setChangeNote("");
    setMetaPicker(null);
  }

  const errors = issues.filter((i) => i.level === "error");
  const warnings = issues.filter((i) => i.level !== "error");
  const canExport = errors.length === 0 && Boolean(destDir) && !busy;
  const linkedWorkshopId = parseWorkshopId(workshopId);
  const canPublish = errors.length === 0 && !busy;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b border-line px-6 py-3">
        <div className="flex items-center gap-3">
          <img
            src="/favicon.png"
            alt=""
            className="h-10 w-10 rounded-md"
          />
          <div>
            <div className="flex items-center gap-2">
              <span className="text-[11px] tracking-[0.28em] text-gold uppercase">
                Garry's Mod
              </span>
              <span
                className="text-[11px] tracking-[0.28em] text-cream uppercase"
                title={`GMod Record Press version ${packageInfo.version}`}
              >
                v{packageInfo.version}
              </span>
            </div>
            <h1 className="font-display text-2xl text-cream">
              GMod Record Press
            </h1>
          </div>
        </div>
        <div className="flex gap-2 text-xs">
          <GhostButton onClick={reset}>New</GhostButton>
          <GhostButton onClick={() => void showOpenPanel()}>Open</GhostButton>
          <GhostButton onClick={() => void saveProject()}>Save</GhostButton>
        </div>
      </header>

      <main className="grid min-h-0 flex-1 grid-cols-[1.15fr_0.85fr] gap-5 overflow-hidden p-5">
        <section className="flex min-h-0 flex-col">
          <div className="mb-3 flex items-center justify-between">
            <ViewSwitch
              value={leftView}
              options={[
                { id: "album", label: "Album" },
                { id: "preview", label: "Preview" },
              ]}
              onChange={setLeftView}
            />
          </div>
          <div className="view-stack">
            <div
              className="view-pane flex flex-col gap-4 pr-1"
              data-active={leftView === "album" ? "true" : "false"}
            >
              <section className="rounded-xl border border-line bg-panel p-4">
                <h2 className="font-display text-xl text-cream">Album</h2>
                <div className="mt-3 grid grid-cols-2 gap-3">
                  <Field label="Artist" value={artist} onChange={setArtist} />
                  <Field label="Album title" value={album} onChange={setAlbum} />
                  <Field
                    label="Vinyl ID"
                    value={vinylId}
                    onChange={(v) => {
                      setIdTouched(true);
                      setVinylId(slugify(v));
                    }}
                    hint="Used in every file path. Keep it unique."
                  />
                  <label className="block">
                    <span className="text-[11px] tracking-[0.18em] text-muted uppercase">
                      Addon title
                    </span>
                    <div className="mt-1 w-full border-b border-line py-1 text-cream">
                      {addonTitle}
                    </div>
                    <span className="mt-1 block text-[11px] text-muted">
                      Always [Working Record Player] Artist - Album
                    </span>
                  </label>
                </div>
              </section>

              <section className="rounded-xl border border-line bg-panel p-4">
                <div className="mb-3 flex items-end justify-between">
                  <h2 className="font-display text-xl text-cream">Artwork</h2>
                  <div className="flex items-center gap-3 text-xs text-muted">
                    <span>Vinyl color</span>
                    <input
                      type="color"
                      value={vinylColor}
                      onChange={(e) => setVinylColor(e.target.value)}
                      className="h-7 w-10 rounded"
                    />
                    {VINYL_COLORS.map((c) => (
                      <button
                        key={c.value}
                        type="button"
                        title={c.name}
                        onClick={() => setVinylColor(c.value)}
                        className="h-5 w-5 rounded-full border border-line"
                        style={{ background: c.value }}
                      />
                    ))}
                  </div>
                </div>
                <div className="grid grid-cols-3 gap-3">
                  <div data-drop="cover">
                    <Dropzone
                      label="Front cover"
                      hint="Square crop for the case"
                      preview={cover}
                      required
                      hot={dropTarget === "cover"}
                      onPick={() => void pickArt("cover")}
                      onClear={() => setCover(null)}
                    />
                  </div>
                  <div data-drop="back">
                    <Dropzone
                      label="Back cover"
                      hint="Uses the front if empty"
                      preview={back}
                      hot={dropTarget === "back"}
                      onPick={() => void pickArt("back")}
                      onClear={() => setBack(null)}
                    />
                  </div>
                  <div data-drop="label">
                    <Dropzone
                      label="Vinyl label"
                      hint="Center sticker"
                      preview={label}
                      hot={dropTarget === "label"}
                      onPick={() => void pickArt("label")}
                      onClear={() => setLabel(null)}
                    />
                  </div>
                </div>
              </section>

              <div data-drop="tracks" className="flex min-h-64 flex-1 flex-col">
                <TrackList
                  tracks={tracks}
                  hot={dropTarget === "tracks"}
                  onAdd={() => void addTracks()}
                  onRename={(id, name) =>
                    setTracks((cur) =>
                      cur.map((t) => (t.id === id ? { ...t, name } : t)),
                    )
                  }
                  onMove={moveTrack}
                  onRemove={(id) =>
                    setTracks((cur) => cur.filter((t) => t.id !== id))
                  }
                />
              </div>
            </div>

            <div
              className="view-pane rounded-xl border border-line bg-panel"
              data-active={leftView === "preview" ? "true" : "false"}
            >
              <Preview
                album={album}
                artist={artist}
                vinylColor={vinylColor}
                cover={cover}
                back={back}
                label={label}
              />
            </div>
          </div>
        </section>

        <aside className="flex min-h-0 flex-col">
          <div className="mb-3 flex items-center justify-between gap-3">
            <ViewSwitch
              value={rightView}
              options={[
                { id: "export", label: "Export" },
                { id: "workshop", label: "Workshop" },
              ]}
              onChange={(view) => {
                setRightView(view);
                if (view === "workshop") void refreshSteam();
              }}
            />
          </div>
          {busy && progress && (
            <div className="mb-3 h-1 overflow-hidden rounded bg-line">
              <div
                className="h-full bg-gold"
                style={{ width: `${progress.percent}%` }}
              />
            </div>
          )}
          <div className="view-stack">
            <div className="view-pane" data-active={rightView === "export" ? "true" : "false"}>
              <section className="h-full overflow-auto rounded-xl border border-line bg-panel p-4">
            <h2 className="font-display text-xl text-cream">Export</h2>
            <p className="mt-1 text-xs text-muted">
              Writes a drop-in addon folder. Put it in garrysmod/addons and spawn the album
              next to the Working Record Player.
            </p>
            <div className="mt-3 flex gap-2">
              <input
                value={destDir}
                onChange={(e) => setDestDir(e.target.value)}
                className="flex-1 rounded-lg border border-line bg-ink px-3 py-2 text-sm outline-none"
                placeholder="Export folder"
              />
              <GhostButton onClick={() => void chooseDest()}>Browse</GhostButton>
            </div>
            {gmodDir && (
              <button
                type="button"
                className="mt-2 text-left text-xs text-gold"
                onClick={() => setDestDir(gmodDir)}
              >
                Use detected GMod addons folder
              </button>
            )}
            <label className="mt-3 flex items-center gap-2 text-sm text-muted">
              Vinyl resolution
              <select
                value={vinylResolution}
                onChange={(e) => setVinylResolution(Number(e.target.value))}
                className="rounded border border-line bg-ink px-2 py-1 text-cream"
              >
                <option value={1024}>1024 — light (194px label)</option>
                <option value={2048}>2048 — default (388px label)</option>
                <option value={4096}>4096 — official size (776px label)</option>
              </select>
            </label>
            <label className="mt-2 flex items-center gap-2 text-sm text-muted">
              <input
                type="checkbox"
                checked={writeGma}
                onChange={(e) => setWriteGma(e.target.checked)}
              />
              Also pack a .gma
            </label>
            <label className="mt-1 flex items-center gap-2 text-sm text-muted">
              <input
                type="checkbox"
                checked={writeIcon}
                onChange={(e) => setWriteIcon(e.target.checked)}
              />
              Write a 512×512 workshop JPEG beside the addon
            </label>

            <button
              type="button"
              disabled={!canExport}
              onClick={() => void exportAlbum()}
              className="mt-4 w-full rounded-full bg-label py-3 font-display text-lg text-cream disabled:opacity-40"
            >
              {busy ? progress?.detail ?? "Exporting…" : "Export album addon"}
            </button>
            {error && <p className="mt-3 text-sm text-label">{error}</p>}
            {result && (
              <div className="mt-3 rounded-lg bg-ink p-3 text-xs text-muted">
                <div className="text-cream">Wrote {result.filesWritten} files</div>
                <button
                  type="button"
                  className="mt-1 text-gold"
                  onClick={() => void api.openPath(result.addonDir)}
                >
                  Open addon folder
                </button>
              </div>
            )}
            <ul className="mt-3 space-y-1 text-xs">
              {errors.map((issue) => (
                <li key={issue.message} className="text-label">
                  {issue.message}
                </li>
              ))}
              {warnings.map((issue) => (
                <li key={issue.message} className="text-muted">
                  {issue.message}
                </li>
              ))}
            </ul>
              </section>
            </div>

            <div className="view-pane" data-active={rightView === "workshop" ? "true" : "false"}>
              <section className="h-full overflow-auto rounded-xl border border-line bg-panel p-4">
            <h2 className="font-display text-xl text-cream">Workshop</h2>
            <p className="mt-1 text-xs text-muted">
              Publishes through Steamworks, same path as gmpublisher. Steam must be
              running and you must own Garry's Mod. New items start private. Subscribers
              are required to have{" "}
              <button
                type="button"
                className="text-gold"
                onClick={() => void openUrl(RECORD_PLAYER_WORKSHOP_URL)}
              >
                Working Record Player
              </button>
              .
            </p>
            <div className="mt-3 flex items-center justify-between gap-2 text-xs">
              <span className={steam?.connected ? "text-gold" : "text-muted"}>
                {steam?.connected
                  ? `Connected as ${steam.persona ?? "Steam"}`
                  : steam?.error
                    ? "Steam is not connected"
                    : "Checking Steam…"}
              </span>
              <GhostButton onClick={() => void refreshSteam()}>Retry</GhostButton>
            </div>
            {steam && !steam.connected && steam.error && (
              <p className="mt-2 text-xs text-label">{steam.error}</p>
            )}

            <label className="mt-3 block">
              <span className="text-[11px] tracking-[0.18em] text-muted uppercase">
                Visibility
              </span>
              <select
                value={workshopVisibility}
                onChange={(e) => setWorkshopVisibility(e.target.value)}
                className="mt-1 w-full rounded-lg border border-line bg-ink px-3 py-2 text-sm text-cream outline-none"
              >
                <option value="private">Private</option>
                <option value="friends">Friends only</option>
                <option value="public">Public</option>
              </select>
            </label>

            <label className="mt-3 flex items-center gap-2 text-sm text-muted">
              <input
                type="checkbox"
                checked={useDescriptionTemplate}
                onChange={(e) => setUseDescriptionTemplate(e.target.checked)}
              />
              Use default description
            </label>
            {!useDescriptionTemplate && (
              <label className="mt-2 block">
                <span className="text-[11px] tracking-[0.18em] text-muted uppercase">
                  Description
                </span>
                <textarea
                  value={workshopDescription}
                  onChange={(e) => setWorkshopDescription(e.target.value)}
                  rows={10}
                  className="mt-1 w-full resize-y rounded-lg border border-line bg-ink px-3 py-2 text-sm text-cream outline-none focus:border-gold"
                />
              </label>
            )}

            <label className="mt-3 block">
              <span className="text-[11px] tracking-[0.18em] text-muted uppercase">
                Workshop ID
              </span>
              <input
                value={workshopId}
                onChange={(e) => setWorkshopId(e.target.value.replace(/[^\d]/g, ""))}
                className="mt-1 w-full rounded-lg border border-line bg-ink px-3 py-2 text-sm text-cream outline-none focus:border-gold"
                placeholder="Leave empty to create a new item"
              />
            </label>
            {workshopItems.length > 0 && (
              <div className="mt-2 block">
                <span className="flex items-center justify-between gap-3">
                  <label
                    htmlFor="workshop-item"
                    className="text-[11px] tracking-[0.18em] text-muted uppercase"
                  >
                    Your addons
                  </label>
                  <label className="flex items-center gap-1.5 text-xs text-muted">
                    <input
                      type="checkbox"
                      checked={showOnlyRelevantWorkshopItems}
                      onChange={(e) => setShowOnlyRelevantWorkshopItems(e.target.checked)}
                    />
                    Show only relevant
                  </label>
                </span>
                <select
                  id="workshop-item"
                  value={workshopId}
                  onChange={(e) => setWorkshopId(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-line bg-ink px-3 py-2 text-sm text-cream outline-none"
                >
                  <option value="">Create a new Workshop item</option>
                  {visibleWorkshopItems.map((item) => (
                    <option key={item.id} value={String(item.id)}>
                      {item.title} ({item.id})
                    </option>
                  ))}
                </select>
              </div>
            )}
            {linkedWorkshopId && (
              <button
                type="button"
                className="mt-2 text-xs text-gold"
                onClick={() => setWorkshopId("")}
              >
                Unlink and create a new item
              </button>
            )}

            {linkedWorkshopId ? (
              <label className="mt-3 block">
                <span className="text-[11px] tracking-[0.18em] text-muted uppercase">
                  Change notes
                </span>
                <input
                  value={changeNote}
                  onChange={(e) => setChangeNote(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-line bg-ink px-3 py-2 text-sm text-cream outline-none focus:border-gold"
                  placeholder="Optional changelog"
                />
              </label>
            ) : null}

            <button
              type="button"
              disabled={!canPublish}
              onClick={() => void publishToWorkshop()}
              className="mt-4 w-full rounded-full border border-gold bg-transparent py-3 font-display text-lg text-gold disabled:opacity-40"
            >
              {busy
                ? progress?.detail ?? "Publishing…"
                : linkedWorkshopId
                  ? "Update Workshop item"
                  : "Publish to Workshop"}
            </button>
            {error && <p className="mt-3 text-sm text-label">{error}</p>}
            {workshopResult && (
              <div className="mt-3 rounded-lg bg-ink p-3 text-xs text-muted">
                <div className="text-cream">
                  {workshopResult.updated ? "Updated" : "Published"} item{" "}
                  {workshopResult.workshopId}
                </div>
                <button
                  type="button"
                  className="mt-1 text-gold"
                  onClick={() => void openUrl(workshopResult.url)}
                >
                  Open Workshop page
                </button>
                {workshopResult.needsLegalAgreement && (
                  <button
                    type="button"
                    className="mt-1 block text-gold"
                    onClick={() => void openUrl(workshopResult.legalAgreementUrl)}
                  >
                    Accept Steam Workshop legal agreement
                  </button>
                )}
                {workshopResult.dependencyError ? (
                  <p className="mt-2 text-label">
                    Uploaded, but Steam did not attach Working Record Player as a
                    required item. Add it on the Workshop page under Required items.
                  </p>
                ) : (
                  <div className="mt-2 text-muted">
                    Requires{" "}
                    <button
                      type="button"
                      className="text-gold"
                      onClick={() => void openUrl(RECORD_PLAYER_WORKSHOP_URL)}
                    >
                      Working Record Player
                    </button>
                  </div>
                )}
              </div>
            )}
              </section>
            </div>
          </div>
        </aside>
      </main>
      {metaPicker && (
        <MetadataPicker
          textConflicts={metaPicker.textConflicts}
          artConflicts={metaPicker.artConflicts}
          onSkip={() => setMetaPicker(null)}
          onApply={(picked) => {
            applyAlbumMeta(picked);
            setMetaPicker(null);
          }}
        />
      )}
      <OpenPanel
        open={openPanel}
        loading={openLoading}
        gmodDetected={Boolean(gmodDir)}
        scannedDir={libraryDir}
        addons={libraryAddons}
        error={openError}
        onClose={() => {
          setOpenPanel(false);
          setOpenError(null);
        }}
        onPickAddon={(path) => void openVinylAddon(path)}
        onBrowseFolder={() => void browseAddonFolder()}
        onOpenProjectFile={() => void openProjectFile()}
      />
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  hint,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  hint?: string;
}) {
  return (
    <label className="block">
      <span className="text-[11px] tracking-[0.18em] text-muted uppercase">{label}</span>
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1 w-full border-b border-line bg-transparent py-1 text-cream outline-none focus:border-gold"
      />
      {hint && <span className="mt-1 block text-[11px] text-muted">{hint}</span>}
    </label>
  );
}

function GhostButton({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded-full border border-line px-3 py-1 text-muted hover:border-gold hover:text-cream"
    >
      {children}
    </button>
  );
}

function targetFromPoint(x: number, y: number): DropTarget {
  const el = document.elementFromPoint(x, y);
  const node = el?.closest("[data-drop]") as HTMLElement | null;
  const value = node?.dataset.drop;
  if (value === "cover" || value === "back" || value === "label" || value === "tracks") {
    return value;
  }
  return "tracks";
}
