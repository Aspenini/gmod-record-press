import { useEffect, useState } from "react";
import { api } from "../lib/api";
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
  const rear = back ?? cover;
  const sticker = label ?? cover;
  const [workshopCover, setWorkshopCover] = useState<ImagePreview | null>(null);

  useEffect(() => {
    let cancelled = false;
    const coverPath = cover?.path;
    if (!coverPath) {
      setWorkshopCover(null);
      return;
    }

    const timer = window.setTimeout(() => {
      void api
        .workshopIconPreview({
          coverPath,
          labelPath: label?.path ?? null,
          vinylColor,
          artist,
          album,
        })
        .then((preview) => {
          if (!cancelled) setWorkshopCover(preview);
        })
        .catch(() => {
          if (!cancelled) setWorkshopCover(null);
        });
    }, 180);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [album, artist, cover?.path, label?.path, vinylColor]);

  return (
    <div className="flex h-full min-h-0 flex-col items-center justify-center gap-8 px-4 py-6">
      <div className="flex flex-wrap items-end justify-center gap-8">
        <CaseFace
          image={cover}
          caption="Front"
          empty="Front cover"
        />
        <VinylFace
          color={vinylColor}
          sticker={sticker}
          spinning={Boolean(cover || label)}
        />
        <CaseFace
          image={rear}
          caption={back ? "Back" : "Back · using front"}
          empty="Back cover"
        />
      </div>
      <WorkshopCover image={workshopCover} />
      <div className="text-center">
        <div className="font-display text-3xl text-cream">
          {album || "Untitled album"}
        </div>
        <div className="text-sm tracking-[0.18em] text-muted uppercase">
          {artist || "Unknown artist"}
        </div>
      </div>
    </div>
  );
}

function WorkshopCover({ image }: { image: ImagePreview | null }) {
  return (
    <figure className="flex flex-col items-center gap-3">
      <div className="relative h-64 w-64 overflow-hidden rounded-sm border border-line shadow-[12px_18px_40px_rgba(0,0,0,0.45)]">
        {image ? (
          <img src={image.dataUrl} alt="Workshop cover" className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full w-full items-center justify-center bg-raised px-8 text-center text-sm text-muted">
            Add a front cover to preview the Workshop image
          </div>
        )}
      </div>
      <figcaption className="text-[11px] tracking-[0.22em] text-gold uppercase">
        Workshop cover
      </figcaption>
    </figure>
  );
}

function CaseFace({
  image,
  caption,
  empty,
}: {
  image: ImagePreview | null;
  caption: string;
  empty: string;
}) {
  return (
    <figure className="flex flex-col items-center gap-3">
      <div
        className="relative h-56 w-56 overflow-hidden rounded-sm shadow-[12px_18px_40px_rgba(0,0,0,0.45)]"
        style={{ transform: "perspective(900px) rotateY(-6deg)" }}
      >
        <div className="absolute inset-y-0 left-0 z-10 w-3 bg-gradient-to-r from-black/55 to-transparent" />
        {image ? (
          <img src={image.dataUrl} alt={caption} className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full w-full items-center justify-center bg-raised text-sm text-muted">
            {empty}
          </div>
        )}
      </div>
      <figcaption className="text-[11px] tracking-[0.22em] text-gold uppercase">
        {caption}
      </figcaption>
    </figure>
  );
}

function VinylFace({
  color,
  sticker,
  spinning,
}: {
  color: string;
  sticker: ImagePreview | null;
  spinning: boolean;
}) {
  return (
    <figure className="flex flex-col items-center gap-3">
      <div
        className="disc-spin relative h-64 w-64 rounded-full shadow-[0_20px_50px_rgba(0,0,0,0.55)]"
        style={{
          background: `
            radial-gradient(circle at 50% 50%, #0a0a0a 0 3.4%, #efe6d4 3.5% 4.6%, transparent 4.7%),
            radial-gradient(circle at 32% 28%, rgba(255,255,255,0.16), transparent 28%),
            repeating-radial-gradient(circle at 50% 50%, rgba(255,255,255,0.035) 0 1px, transparent 1px 3px),
            ${color}
          `,
          animationPlayState: spinning ? "running" : "paused",
        }}
      >
        <div
          className="absolute left-1/2 top-1/2 overflow-hidden rounded-full"
          style={{
            width: "32.6%",
            height: "32.6%",
            transform: "translate(-50%, -50%)",
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
      <figcaption className="text-[11px] tracking-[0.22em] text-gold uppercase">
        Vinyl
      </figcaption>
    </figure>
  );
}
