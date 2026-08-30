export function slugify(input: string): string {
  const slug = input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return slug || "album";
}

export function defaultAddonTitle(artist: string, album: string): string {
  const a = artist.trim();
  const b = album.trim();
  if (!a && !b) return "";
  if (!a) return `[Working Record Player] ${b}`;
  if (!b) return `[Working Record Player] ${a}`;
  return `[Working Record Player] ${a} - ${b}`;
}

export function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(0)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

export function parseWorkshopId(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  if (!/^\d+$/.test(trimmed)) return null;
  try {
    const id = BigInt(trimmed);
    if (id <= 0n || id > 18446744073709551615n) return null;
    return Number(id);
  } catch {
    return null;
  }
}
