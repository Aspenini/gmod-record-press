import { useEffect } from "react";
import type { VinylAddonInfo } from "../types";

type Props = {
  open: boolean;
  loading: boolean;
  gmodDetected: boolean;
  scannedDir: string | null;
  addons: VinylAddonInfo[];
  error: string | null;
  onClose: () => void;
  onPickAddon: (path: string) => void;
  onBrowseFolder: () => void;
  onOpenProjectFile: () => void;
};

export function OpenPanel({
  open,
  loading,
  gmodDetected,
  scannedDir,
  addons,
  error,
  onClose,
  onPickAddon,
  onBrowseFolder,
  onOpenProjectFile,
}: Props) {
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/80 p-6"
      onClick={onClose}
    >
      <div
        className="flex max-h-[min(720px,90vh)] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border border-line bg-panel shadow-[0_24px_80px_rgba(0,0,0,0.55)]"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-start justify-between border-b border-line px-5 py-4">
          <div>
            <h2 className="font-display text-2xl text-cream">Open vinyl</h2>
            <p className="mt-1 text-xs text-muted">
              {scannedDir
                ? `Record player albums in ${scannedDir}`
                : gmodDetected
                  ? "Record player albums in your Garry's Mod addons folder"
                  : "Garry's Mod was not detected on this computer"}
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-line px-3 py-1 text-xs text-muted hover:border-gold hover:text-cream"
          >
            Close
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-auto p-5">
          {error && <p className="mb-3 text-sm text-label">{error}</p>}
          {loading ? (
            <p className="py-12 text-center text-sm text-muted">Looking for albums…</p>
          ) : addons.length ? (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              {addons.map((addon) => (
                <button
                  key={addon.path}
                  type="button"
                  onClick={() => onPickAddon(addon.path)}
                  className="overflow-hidden rounded-xl border border-line bg-ink text-left hover:border-gold"
                >
                  {addon.coverDataUrl ? (
                    <img
                      src={addon.coverDataUrl}
                      alt=""
                      className="h-32 w-full object-cover"
                    />
                  ) : (
                    <div className="flex h-32 items-center justify-center bg-raised text-xs text-muted">
                      No cover
                    </div>
                  )}
                  <div className="px-3 py-2">
                    <div className="truncate font-display text-cream">
                      {addon.album || addon.folderName}
                    </div>
                    <div className="truncate text-xs text-muted">
                      {addon.artist || "Unknown artist"}
                    </div>
                    <div className="mt-1 text-[11px] text-gold">
                      {addon.trackCount} track{addon.trackCount === 1 ? "" : "s"}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div className="rounded-xl border border-dashed border-line bg-ink/40 px-6 py-12 text-center">
              <div className="text-[11px] tracking-[0.22em] text-gold uppercase">
                {gmodDetected || scannedDir ? "Empty" : "No GMod detected"}
              </div>
              <p className="mt-2 font-display text-xl text-cream">
                {gmodDetected || scannedDir
                  ? "No record player albums here"
                  : "Garry's Mod was not detected"}
              </p>
              <p className="mt-2 text-sm text-muted">
                {gmodDetected || scannedDir
                  ? "Export a vinyl into addons, or choose a folder if you saved it somewhere else."
                  : "Choose the folder you exported to, or install Garry's Mod and try again."}
              </p>
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line px-5 py-3">
          <button
            type="button"
            onClick={onOpenProjectFile}
            className="text-xs text-muted hover:text-gold"
          >
            Open project file
          </button>
          <button
            type="button"
            onClick={onBrowseFolder}
            className="rounded-full border border-gold px-4 py-2 text-sm text-gold hover:bg-gold/10"
          >
            Choose folder
          </button>
        </div>
      </div>
    </div>
  );
}
