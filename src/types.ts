export type Issue = {
  level: "error" | "warning" | string;
  message: string;
};

export type ImagePreview = {
  path: string;
  dataUrl: string;
  width: number;
  height: number;
};

export type AudioInfo = {
  path: string;
  fileName: string;
  suggestedName: string;
  size: number;
};

export type Track = {
  id: string;
  name: string;
  path: string;
  fileName: string;
  size: number;
};

export type AlbumProject = {
  artist: string;
  album: string;
  vinylId: string;
  addonTitle: string;
  coverPath: string | null;
  backCoverPath: string | null;
  labelPath: string | null;
  vinylColor: string;
  vinylResolution: number;
  tracks: { name: string; path: string }[];
};

export type ExportOptions = {
  destDir: string;
  writeGma: boolean;
  writeWorkshopIcon: boolean;
};

export type ExportProgress = {
  stage: string;
  detail: string;
  percent: number;
};

export type ExportResult = {
  addonDir: string;
  gmaPath: string | null;
  workshopIconPath: string | null;
  filesWritten: number;
};

export const VINYL_COLORS = [
  { name: "Black", value: "#141414" },
  { name: "Oxblood", value: "#4a1218" },
  { name: "Coke bottle", value: "#8aa39b" },
  { name: "White", value: "#e8e2d6" },
  { name: "Navy", value: "#1b2b4a" },
  { name: "Gold", value: "#b68a3a" },
];
