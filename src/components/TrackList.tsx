import type { Track } from "../types";
import { formatBytes } from "../lib/slug";

type Props = {
  tracks: Track[];
  hot?: boolean;
  onAdd: () => void;
  onRename: (id: string, name: string) => void;
  onMove: (id: string, dir: -1 | 1) => void;
  onRemove: (id: string) => void;
};

export function TrackList({
  tracks,
  hot,
  onAdd,
  onRename,
  onMove,
  onRemove,
}: Props) {
  return (
    <section
      className={`flex min-h-0 flex-1 flex-col rounded-xl border bg-panel ${
        hot ? "border-gold" : "border-line"
      }`}
    >
      <header className="flex items-center justify-between border-b border-line px-4 py-3">
        <div>
          <h2 className="font-display text-xl text-cream">Tracks</h2>
          <p className="text-xs text-muted">
            Drop mp3 / ogg / wav files, then name them as they should appear in-game.
          </p>
        </div>
        <button
          type="button"
          onClick={onAdd}
          className="rounded-full border border-gold/40 px-3 py-1 text-xs tracking-wide text-gold uppercase hover:bg-gold/10"
        >
          Add files
        </button>
      </header>
      <ol className="min-h-0 flex-1 overflow-auto p-2">
        {tracks.length === 0 && (
          <li className="px-3 py-10 text-center text-sm text-muted">
            No tracks yet. A vinyl needs at least one song.
          </li>
        )}
        {tracks.map((track, index) => (
          <li
            key={track.id}
            className="mb-1 grid grid-cols-[2.2rem_1fr_auto] items-center gap-2 rounded-lg px-2 py-2 hover:bg-raised"
          >
            <span className="font-display text-sm text-gold">
              {String(index + 1).padStart(2, "0")}
            </span>
            <div className="min-w-0">
              <input
                value={track.name}
                onChange={(e) => onRename(track.id, e.target.value)}
                className="w-full bg-transparent text-sm text-cream outline-none"
              />
              <div className="truncate text-[11px] text-muted">
                {track.fileName} · {formatBytes(track.size)}
              </div>
            </div>
            <div className="flex items-center gap-1 text-xs text-muted">
              <button type="button" onClick={() => onMove(track.id, -1)}>
                ↑
              </button>
              <button type="button" onClick={() => onMove(track.id, 1)}>
                ↓
              </button>
              <button
                type="button"
                className="text-label"
                onClick={() => onRemove(track.id)}
              >
                ✕
              </button>
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
