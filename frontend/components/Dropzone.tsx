import type { ImagePreview } from "../types";

type Props = {
  label: string;
  hint: string;
  preview: ImagePreview | null;
  required?: boolean;
  hot?: boolean;
  onPick: () => void;
  onClear?: () => void;
};

export function Dropzone({
  label,
  hint,
  preview,
  required,
  hot,
  onPick,
  onClear,
}: Props) {
  return (
    <button
      type="button"
      onClick={onPick}
      className={`group relative flex min-h-36 flex-col overflow-hidden rounded-xl border text-left transition ${
        hot
          ? "border-gold bg-gold/10"
          : preview
            ? "border-line bg-raised"
            : "border-dashed border-line bg-ink/40 hover:border-gold/50"
      }`}
    >
      {preview ? (
        <img
          src={preview.dataUrl}
          alt={label}
          className="h-36 w-full object-cover"
        />
      ) : (
        <div className="flex h-36 flex-col items-center justify-center gap-1 px-3 text-center">
          <span className="text-xs tracking-[0.18em] text-gold uppercase">
            {required ? "Required" : "Optional"}
          </span>
          <span className="font-display text-lg text-cream">{label}</span>
          <span className="text-xs text-muted">{hint}</span>
        </div>
      )}
      {preview && (
        <div className="absolute inset-x-0 bottom-0 flex items-center justify-between bg-ink/75 px-3 py-2 text-xs">
          <span className="truncate text-cream">{label}</span>
          {onClear && (
            <span
              className="text-gold hover:text-cream"
              onClick={(e) => {
                e.stopPropagation();
                onClear();
              }}
            >
              Remove
            </span>
          )}
        </div>
      )}
    </button>
  );
}
