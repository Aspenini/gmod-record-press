import type { AudioInfo, EmbeddedArt, ImagePreview } from "../types";

export type TextField = "artist" | "album";
export type ArtSlot = "cover" | "back" | "label";

export type TextConflict = {
  field: TextField;
  label: string;
  options: { value: string; count: number }[];
};

export type ArtConflict = {
  field: ArtSlot;
  label: string;
  options: { art: EmbeddedArt; count: number }[];
};

export type AlbumMetaSuggestion = {
  artist?: string;
  album?: string;
  cover?: ImagePreview;
  back?: ImagePreview;
  label?: ImagePreview;
  textConflicts: TextConflict[];
  artConflicts: ArtConflict[];
};

export type CurrentAlbumMeta = {
  artist: string;
  album: string;
  hasCover: boolean;
  hasBack: boolean;
  hasLabel: boolean;
};

export function artToPreview(art: EmbeddedArt): ImagePreview {
  return {
    path: art.path,
    dataUrl: art.dataUrl,
    width: art.width,
    height: art.height,
  };
}

export function suggestAlbumMeta(
  tracks: AudioInfo[],
  pictures: EmbeddedArt[],
  current: CurrentAlbumMeta,
): AlbumMetaSuggestion {
  const byId = new Map(pictures.map((art) => [art.id, art]));
  const suggestion: AlbumMetaSuggestion = {
    textConflicts: [],
    artConflicts: [],
  };

  if (!current.artist.trim()) {
    applyText(suggestion, "artist", "Artist", tracks.map(albumLevelArtist));
  }
  if (!current.album.trim()) {
    applyText(
      suggestion,
      "album",
      "Album title",
      tracks.map((track) => clean(track.album)),
    );
  }
  if (!current.hasCover) {
    applyArt(suggestion, "cover", "Front cover", coverCandidates(tracks, byId));
  }
  if (!current.hasBack) {
    applyArt(suggestion, "back", "Back cover", slotCandidates(tracks, byId, "back"));
  }
  if (!current.hasLabel) {
    applyArt(suggestion, "label", "Vinyl label", slotCandidates(tracks, byId, "label"));
  }

  return suggestion;
}

export function albumLevelArtist(track: AudioInfo): string | null {
  return clean(track.albumArtist) ?? clean(track.artist);
}

function applyText(
  suggestion: AlbumMetaSuggestion,
  field: TextField,
  label: string,
  values: (string | null)[],
) {
  const options = tally(values);
  if (options.length === 1) {
    if (field === "artist") suggestion.artist = options[0].value;
    if (field === "album") suggestion.album = options[0].value;
  } else if (options.length > 1) {
    suggestion.textConflicts.push({ field, label, options });
  }
}

function applyArt(
  suggestion: AlbumMetaSuggestion,
  field: ArtSlot,
  label: string,
  options: { art: EmbeddedArt; count: number }[],
) {
  if (options.length === 1) {
    const preview = artToPreview(options[0].art);
    if (field === "cover") suggestion.cover = preview;
    if (field === "back") suggestion.back = preview;
    if (field === "label") suggestion.label = preview;
  } else if (options.length > 1) {
    suggestion.artConflicts.push({ field, label, options });
  }
}

function coverCandidates(
  tracks: AudioInfo[],
  byId: Map<string, EmbeddedArt>,
): { art: EmbeddedArt; count: number }[] {
  const fronts = slotCandidates(tracks, byId, "front");
  if (fronts.length) return fronts;
  return slotCandidates(tracks, byId, "other");
}

function slotCandidates(
  tracks: AudioInfo[],
  byId: Map<string, EmbeddedArt>,
  kind: string,
): { art: EmbeddedArt; count: number }[] {
  const counts = new Map<string, number>();
  for (const track of tracks) {
    const seen = new Set<string>();
    for (const picture of track.pictures ?? []) {
      if (picture.kind !== kind || seen.has(picture.id)) continue;
      seen.add(picture.id);
      counts.set(picture.id, (counts.get(picture.id) ?? 0) + 1);
    }
  }
  return [...counts.entries()]
    .map(([id, count]) => {
      const art = byId.get(id);
      return art ? { art, count } : null;
    })
    .filter((entry): entry is { art: EmbeddedArt; count: number } => entry !== null)
    .sort((a, b) => b.count - a.count || a.art.id.localeCompare(b.art.id));
}

function tally(values: (string | null)[]): { value: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const value of values) {
    if (!value) continue;
    counts.set(value, (counts.get(value) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([value, count]) => ({ value, count }))
    .sort((a, b) => b.count - a.count || a.value.localeCompare(b.value));
}

function clean(value: string | null | undefined): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}
