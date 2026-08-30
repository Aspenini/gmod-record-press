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

export type TrackPicture = {
  id: string;
  kind: string;
};

export type EmbeddedArt = {
  id: string;
  kind: string;
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
  artist?: string | null;
  album?: string | null;
  albumArtist?: string | null;
  title?: string | null;
  trackNumber?: number | null;
  pictures?: TrackPicture[];
};

export type AudioScan = {
  tracks: AudioInfo[];
  pictures: EmbeddedArt[];
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
  workshopId: number | null;
  workshopDescription: string;
  workshopVisibility: "private" | "friends" | "public" | string;
  workshopUseTemplate: boolean;
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

export type VinylAddonInfo = {
  path: string;
  folderName: string;
  vinylId: string;
  artist: string;
  album: string;
  addonTitle: string;
  trackCount: number;
  coverDataUrl: string | null;
};

export type VinylLibrary = {
  gmodAddonsDir: string | null;
  scannedDir: string | null;
  addons: VinylAddonInfo[];
};

export type WorkshopStatus = {
  connected: boolean;
  persona: string | null;
  error: string | null;
};

export type WorkshopItem = {
  id: number;
  title: string;
};

export type WorkshopPublishOptions = {
  destDir: string;
  workshopId: number | null;
  description: string;
  visibility: string;
  changeNote: string;
  useTemplate: boolean;
};

export type WorkshopPublishResult = {
  workshopId: number;
  url: string;
  updated: boolean;
  needsLegalAgreement: boolean;
  legalAgreementUrl: string;
  export: ExportResult;
};

export const VINYL_COLORS = [
  { name: "Black", value: "#141414" },
  { name: "Oxblood", value: "#4a1218" },
  { name: "Coke bottle", value: "#8aa39b" },
  { name: "White", value: "#e8e2d6" },
  { name: "Navy", value: "#1b2b4a" },
  { name: "Gold", value: "#b68a3a" },
];
