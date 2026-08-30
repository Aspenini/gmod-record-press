import { useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Dropzone } from "./components/Dropzone";
import { Preview } from "./components/Preview";
import { TrackList } from "./components/TrackList";
import { api } from "./lib/api";
import { defaultAddonTitle, slugify } from "./lib/slug";
import type {
  AlbumProject,
  AudioInfo,
  ExportProgress,
  ExportResult,
  ImagePreview,
  Issue,
  Track,
} from "./types";
import { VINYL_COLORS } from "./types";

const IMAGE_EXT = [".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tga"];
const AUDIO_EXT = [".mp3", ".ogg", ".wav"];

type DropTarget = "cover" | "back" | "label" | "tracks" | null;

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
  const [addonTitle, setAddonTitle] = useState("");
  const [titleTouched, setTitleTouched] = useState(false);
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
  const [issues, setIssues] = useState<Issue[]>([]);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<ExportProgress | null>(null);
  const [result, setResult] = useState<ExportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget>(null);
  const [projectPath, setProjectPath] = useState<string | null>(null);

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
    ],
  );

  useEffect(() => {
    api.suggestGmodAddonsDir().then((dir) => {
      setGmodDir(dir);
      const last = localStorage.getItem("rpam.destDir");
      if (last) setDestDir(last);
      else if (dir) setDestDir(dir);
    });
  }, []);

  useEffect(() => {
    if (!idTouched) setVinylId(album.trim() ? slugify(album) : "");
  }, [album, idTouched]);

  useEffect(() => {
    if (!titleTouched) setAddonTitle(defaultAddonTitle(artist, album));
  }, [artist, album, titleTouched]);

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

    listen<ExportProgress>("export-progress", (event) => {
      if (!cancelled) setProgress(event.payload);
    }).then((fn) => {
      unlistenProgress = fn;
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
          void handleDroppedPaths(event.payload.paths, target);
        }
      })
      .then((fn) => {
        unlistenDrop = fn;
      });

    return () => {
      cancelled = true;
      unlistenDrop?.();
      unlistenProgress?.();
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
      const infos = await api.audioInfo(audio);
      setTracks((cur) => {
        const known = new Set(cur.map((t) => t.path));
        return [...cur, ...infos.filter((i) => !known.has(i.path)).map(newTrack)];
      });
    }

    if (!images.length) return;
    const preview = await api.readImagePreview(images[0]);
    if (target === "back") setBack(preview);
    else if (target === "label") setLabel(preview);
    else setCover(preview);
  }

  async function pickArt(which: "cover" | "back" | "label") {
    const picked = await api.pickImage();
    if (!picked) return;
    if (which === "cover") setCover(picked);
    if (which === "back") setBack(picked);
    if (which === "label") setLabel(picked);
  }

  async function addTracks() {
    const infos = await api.pickAudioFiles();
    setTracks((cur) => {
      const known = new Set(cur.map((t) => t.path));
      return [...cur, ...infos.filter((i) => !known.has(i.path)).map(newTrack)];
    });
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

  async function exportAlbum() {
    setError(null);
    setResult(null);
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

  async function saveProject() {
    const path = projectPath ?? (await api.pickSaveProject());
    if (!path) return;
    await api.saveProject(path, project);
    setProjectPath(path);
  }

  async function openProject() {
    const path = await api.pickOpenProject();
    if (!path) return;
    const loaded = await api.loadProject(path);
    setArtist(loaded.artist);
    setAlbum(loaded.album);
    setVinylId(loaded.vinylId);
    setIdTouched(true);
    setAddonTitle(loaded.addonTitle);
    setTitleTouched(Boolean(loaded.addonTitle));
    setVinylColor(loaded.vinylColor);
    setVinylResolution(loaded.vinylResolution);
    setCover(loaded.coverPath ? await api.readImagePreview(loaded.coverPath) : null);
    setBack(
      loaded.backCoverPath ? await api.readImagePreview(loaded.backCoverPath) : null,
    );
    setLabel(loaded.labelPath ? await api.readImagePreview(loaded.labelPath) : null);
    const infos = loaded.tracks.length
      ? await api.audioInfo(loaded.tracks.map((t) => t.path))
      : [];
    setTracks(
      loaded.tracks.map((track, i) => ({
        id: crypto.randomUUID(),
        name: track.name,
        path: track.path,
        fileName: infos[i]?.fileName ?? track.path.split(/[\\/]/).pop() ?? "track",
        size: infos[i]?.size ?? 0,
      })),
    );
    setProjectPath(path);
    setResult(null);
  }

  function reset() {
    setArtist("");
    setAlbum("");
    setVinylId("");
    setIdTouched(false);
    setAddonTitle("");
    setTitleTouched(false);
    setCover(null);
    setBack(null);
    setLabel(null);
    setVinylColor("#141414");
    setVinylResolution(2048);
    setTracks([]);
    setResult(null);
    setError(null);
    setProjectPath(null);
  }

  const errors = issues.filter((i) => i.level === "error");
  const warnings = issues.filter((i) => i.level !== "error");
  const canExport = errors.length === 0 && Boolean(destDir) && !busy;

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center justify-between border-b border-line px-6 py-3">
        <div>
          <div className="text-[11px] tracking-[0.28em] text-gold uppercase">
            Garry's Mod
          </div>
          <h1 className="font-display text-2xl text-cream">
            GMod Record Press
          </h1>
        </div>
        <div className="flex gap-2 text-xs">
          <GhostButton onClick={reset}>New</GhostButton>
          <GhostButton onClick={() => void openProject()}>Open</GhostButton>
          <GhostButton onClick={() => void saveProject()}>Save</GhostButton>
        </div>
      </header>

      <main className="grid min-h-0 flex-1 grid-cols-[1.15fr_0.85fr] gap-5 overflow-hidden p-5">
        <div className="flex min-h-0 flex-col gap-4 overflow-auto pr-1">
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
              <Field
                label="Addon title"
                value={addonTitle}
                onChange={(v) => {
                  setTitleTouched(true);
                  setAddonTitle(v);
                }}
              />
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
                setTracks((cur) => cur.map((t) => (t.id === id ? { ...t, name } : t)))
              }
              onMove={moveTrack}
              onRemove={(id) => setTracks((cur) => cur.filter((t) => t.id !== id))}
            />
          </div>
        </div>

        <aside className="flex min-h-0 flex-col gap-4 overflow-auto">
          <section className="rounded-xl border border-line bg-panel p-5">
            <Preview
              album={album}
              artist={artist}
              vinylColor={vinylColor}
              cover={cover}
              back={back}
              label={label}
            />
          </section>

          <section className="rounded-xl border border-line bg-panel p-4">
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
                <option value={1024}>1024 — light</option>
                <option value={2048}>2048 — default</option>
                <option value={4096}>4096 — official size</option>
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
            {busy && progress && (
              <div className="mt-2 h-1 overflow-hidden rounded bg-line">
                <div
                  className="h-full bg-gold"
                  style={{ width: `${progress.percent}%` }}
                />
              </div>
            )}
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
        </aside>
      </main>
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
