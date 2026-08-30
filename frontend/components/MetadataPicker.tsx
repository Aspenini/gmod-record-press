import { useEffect, useMemo, useState } from "react";
import type { ArtConflict, TextConflict } from "../lib/audioMeta";
import type { EmbeddedArt, ImagePreview } from "../types";

type Props = {
  textConflicts: TextConflict[];
  artConflicts: ArtConflict[];
  onApply: (picked: {
    artist?: string;
    album?: string;
    cover?: ImagePreview;
    back?: ImagePreview;
    label?: ImagePreview;
  }) => void;
  onSkip: () => void;
};

export function MetadataPicker({
  textConflicts,
  artConflicts,
  onApply,
  onSkip,
}: Props) {
  const [textPicks, setTextPicks] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      textConflicts.map((conflict) => [conflict.field, conflict.options[0]?.value ?? ""]),
    ),
  );
  const [artPicks, setArtPicks] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      artConflicts.map((conflict) => [conflict.field, conflict.options[0]?.art.id ?? ""]),
    ),
  );

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onSkip();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onSkip]);

  const artByField = useMemo(() => {
    const map = new Map<string, EmbeddedArt>();
    for (const conflict of artConflicts) {
      for (const option of conflict.options) {
        map.set(`${conflict.field}:${option.art.id}`, option.art);
      }
    }
    return map;
  }, [artConflicts]);

  function apply() {
    const picked: {
      artist?: string;
      album?: string;
      cover?: ImagePreview;
      back?: ImagePreview;
      label?: ImagePreview;
    } = {};
    const artist = textPicks.artist?.trim();
    const album = textPicks.album?.trim();
    if (artist) picked.artist = artist;
    if (album) picked.album = album;
    for (const field of ["cover", "back", "label"] as const) {
      const id = artPicks[field];
      if (!id) continue;
      const art = artByField.get(`${field}:${id}`);
      if (!art) continue;
      const preview = {
        path: art.path,
        dataUrl: art.dataUrl,
        width: art.width,
        height: art.height,
      };
      picked[field] = preview;
    }
    onApply(picked);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-ink/80 p-6"
      onClick={onSkip}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="metadata-picker-title"
        className="flex max-h-[min(760px,92vh)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-line bg-panel shadow-[0_24px_80px_rgba(0,0,0,0.55)]"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="border-b border-line px-5 py-4">
          <h2 id="metadata-picker-title" className="font-display text-2xl text-cream">
            Choose album info
          </h2>
          <p className="mt-1 text-xs text-muted">
            These tracks disagree on some tags. Pick one value for each field, or skip
            and fill it yourself.
          </p>
        </div>

        <div className="min-h-0 flex-1 space-y-5 overflow-auto p-5">
          {textConflicts.map((conflict) => (
            <section key={conflict.field}>
              <div className="mb-2 text-[11px] tracking-[0.18em] text-muted uppercase">
                {conflict.label}
              </div>
              <div className="flex flex-col gap-2">
                {conflict.options.map((option) => {
                  const selected = textPicks[conflict.field] === option.value;
                  return (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() =>
                        setTextPicks((cur) => ({ ...cur, [conflict.field]: option.value }))
                      }
                      className={`rounded-xl border px-3 py-2 text-left ${
                        selected
                          ? "border-gold bg-gold/10 text-cream"
                          : "border-line bg-ink text-cream hover:border-gold/50"
                      }`}
                    >
                      <div className="truncate font-display text-lg">{option.value}</div>
                      <div className="text-[11px] text-muted">
                        {option.count} track{option.count === 1 ? "" : "s"}
                      </div>
                    </button>
                  );
                })}
                <button
                  type="button"
                  onClick={() =>
                    setTextPicks((cur) => ({ ...cur, [conflict.field]: "" }))
                  }
                  className={`rounded-xl border border-dashed px-3 py-2 text-left text-sm ${
                    !textPicks[conflict.field]
                      ? "border-gold text-gold"
                      : "border-line text-muted hover:border-gold/50"
                  }`}
                >
                  Leave empty
                </button>
              </div>
            </section>
          ))}

          {artConflicts.map((conflict) => (
            <section key={conflict.field}>
              <div className="mb-2 text-[11px] tracking-[0.18em] text-muted uppercase">
                {conflict.label}
              </div>
              <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                {conflict.options.map((option) => {
                  const selected = artPicks[conflict.field] === option.art.id;
                  return (
                    <button
                      key={option.art.id}
                      type="button"
                      onClick={() =>
                        setArtPicks((cur) => ({ ...cur, [conflict.field]: option.art.id }))
                      }
                      className={`overflow-hidden rounded-xl border text-left ${
                        selected ? "border-gold" : "border-line hover:border-gold/50"
                      }`}
                    >
                      <img
                        src={option.art.dataUrl}
                        alt=""
                        className="h-28 w-full object-cover"
                      />
                      <div className="bg-ink px-2 py-1.5 text-[11px] text-muted">
                        {option.count} track{option.count === 1 ? "" : "s"} ·{" "}
                        {option.art.width}×{option.art.height}
                      </div>
                    </button>
                  );
                })}
                <button
                  type="button"
                  onClick={() =>
                    setArtPicks((cur) => ({ ...cur, [conflict.field]: "" }))
                  }
                  className={`flex h-full min-h-28 items-center justify-center rounded-xl border border-dashed px-3 text-sm ${
                    !artPicks[conflict.field]
                      ? "border-gold text-gold"
                      : "border-line text-muted hover:border-gold/50"
                  }`}
                >
                  Leave empty
                </button>
              </div>
            </section>
          ))}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <button
            type="button"
            onClick={onSkip}
            className="rounded-full border border-line px-4 py-2 text-sm text-muted hover:border-gold hover:text-cream"
          >
            Skip
          </button>
          <button
            type="button"
            onClick={apply}
            className="rounded-full border border-gold bg-gold/10 px-4 py-2 text-sm text-gold hover:bg-gold/20"
          >
            Use selected
          </button>
        </div>
      </div>
    </div>
  );
}
