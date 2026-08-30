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
  if (a && b) return `[Working Record Player] ${a} - ${b}`;
  if (a) return `[Working Record Player] ${a}`;
  if (b) return `[Working Record Player] ${b}`;
  return "[Working Record Player]";
}

export function defaultWorkshopDescription(artist: string, album: string): string {
  const a = artist.trim();
  const b = album.trim();
  const pack = a && b ? `${a} - ${b}` : a || b || "Music Pack";
  return `[h1]Working Record Player - ${pack}[/h1]

Adds music for use with the [b]Working Record Player[/b] addon in Garry's Mod.

[h2]Copyright Notice[/h2]

This is an unofficial, fan-made Workshop addon.

I do not claim ownership of any music, recordings, artwork, trademarks, or other copyrighted material included in this addon. All rights belong to their respective artists, labels, publishers, and copyright holders.

This addon is not affiliated with or endorsed by the original artists or rights holders and is provided for entertainment purposes only.`;
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
