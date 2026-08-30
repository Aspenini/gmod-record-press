type Option<T extends string> = {
  id: T;
  label: string;
};

type Props<T extends string> = {
  value: T;
  options: Option<T>[];
  onChange: (value: T) => void;
};

export function ViewSwitch<T extends string>({ value, options, onChange }: Props<T>) {
  return (
    <div className="inline-flex rounded-full border border-line bg-ink p-1">
      {options.map((option) => {
        const active = option.id === value;
        return (
          <button
            key={option.id}
            type="button"
            onClick={() => onChange(option.id)}
            className={`rounded-full px-3 py-1 text-[11px] tracking-[0.18em] uppercase transition ${
              active ? "bg-gold text-ink" : "text-muted hover:text-cream"
            }`}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
