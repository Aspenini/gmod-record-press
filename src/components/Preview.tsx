import { useState } from "react";
import type { ImagePreview } from "../types";

type Props = {
  album: string;
  artist: string;
  vinylColor: string;
  cover: ImagePreview | null;
  back: ImagePreview | null;
  label: ImagePreview | null;
};

export function Preview({ album, artist, vinylColor, cover, back, label }: Props) {
  const [flipped, setFlipped] = useState(false);
  const sleeve = flipped ? back ?? cover : cover;
  const sticker = label ?? cover;

  return (
    <div className="flex flex-col items-center gap-6">
      <div className="relative h-64 w-64" style={{ perspective: "900px" }}>
        <div className={`sleeve absolute inset-0 ${flipped ? "flipped" : ""}`}>
          <div
            className="absolute inset-0 overflow-hidden rounded-sm shadow-[12px_18px_40px_rgba(0,0,0,0.45)]"
            style={{ backfaceVisibility: "hidden" }}
          >
            <div className="absolute inset-y-0 left-0 w-3 bg-gradient-to-r from-black/50 to-transparent" />
            {sleeve ? (
              <img src={sleeve.dataUrl} alt="Case" className="h-full w-full object-cover" />
            ) : (
              <div className="flex h-full w-full items-center justify-center bg-raised text-sm text-muted">
                Front cover
              </div>
            )}
          </div>
        </div>
      </div>
      <button
        type="button"
        onClick={() => setFlipped((v) => !v)}
        className="text-xs tracking-[0.2em] text-gold uppercase"
      >
        {flipped ? "Show front" : "Flip case"}
      </button>

      <div className="relative">
        <div
          className={`disc-spin relative h-72 w-72 rounded-full shadow-[0_20px_50px_rgba(0,0,0,0.55)] ${
            cover ? "" : "pause"
          }`}
          style={{
            background: `
              radial-gradient(circle at 50% 50%, #0a0a0a 0 3.4%, #efe6d4 3.5% 4.6%, transparent 4.7%),
              radial-gradient(circle at 32% 28%, rgba(255,255,255,0.16), transparent 28%),
              repeating-radial-gradient(circle at 50% 50%, rgba(255,255,255,0.035) 0 1px, transparent 1px 3px),
              ${vinylColor}
            `,
            animationPlayState: cover ? "running" : "paused",
          }}
        >
          <div
            className="absolute left-1/2 top-1/2 overflow-hidden rounded-full"
            style={{
              width: "35%",
              height: "35%",
              transform: "translate(-50%, -50%)",
              boxShadow: "0 0 0 4px #efe6d4",
            }}
          >
            {sticker ? (
              <img src={sticker.dataUrl} alt="Label" className="h-full w-full object-cover" />
            ) : (
              <div className="flex h-full w-full items-center justify-center bg-[#efe6d4] px-2 text-center text-[10px] text-ink">
                Label
              </div>
            )}
          </div>
          <div className="absolute left-1/2 top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-ink" />
        </div>
      </div>

      <div className="text-center">
        <div className="font-display text-2xl text-cream">{album || "Untitled album"}</div>
        <div className="text-sm tracking-[0.16em] text-muted uppercase">
          {artist || "Unknown artist"}
        </div>
      </div>
    </div>
  );
}
