export type ImageFormat = 'png' | 'jpeg' | 'webp';
export type CaptureKind = 'region' | 'fullscreen' | 'window';
export type UploaderKind = 'none' | 'imgur' | 'http' | 'command';
export type CopyFormat = 'url' | 'markdown' | 'html';
export type RecordFormat = 'mp4' | 'webm' | 'gif';
export type AudioMode = 'none' | 'system' | 'microphone' | 'both';

export interface Hotkeys {
  enabled: boolean;
  region: string;
  fullscreen: string;
  window: string;
}

export interface UploadConfig {
  uploader: UploaderKind;
  copyFormat: CopyFormat;
  imgurClientId: string;
  httpUrl: string;
  httpField: string;
  httpHeaders: string;
  httpResponsePath: string;
  command: string;
}

export interface RecordConfig {
  format: RecordFormat;
  fps: number;
  gifFps: number;
  audio: AudioMode;
  systemDevice: string;
  microphoneDevice: string;
}

export interface Config {
  saveDirectory: string;
  imageFormat: ImageFormat;
  jpegQuality: number;
  filenameTemplate: string;
  copyToClipboard: boolean;
  openEditorAfterCapture: boolean;
  showNotification: boolean;
  maxHistory: number;
  captureBackend: string;
  hotkeys: Hotkeys;
  upload: UploadConfig;
  record: RecordConfig;
}

export interface HistoryEntry {
  id: string;
  path: string;
  fileName: string;
  kind: string;
  width: number;
  height: number;
  bytes: number;
  backend: string;
  createdAt: string;
  url?: string | null;
}

export interface CaptureResult extends HistoryEntry {
  copied: boolean;
}

export interface RecordingInfo {
  path: string;
  kind: string;
  format: RecordFormat;
  backend: string;
  startedAt: string;
  width: number;
  height: number;
}

export interface RecordingResult {
  id: string;
  path: string;
  fileName: string;
  kind: string;
  format: RecordFormat;
  width: number;
  height: number;
  bytes: number;
  durationMs: number;
  backend: string;
  createdAt: string;
}

export interface AudioDevice {
  name: string;
  description: string;
  monitor: boolean;
}

export interface UploadOutcome {
  url: string;
  link: string;
  copied: boolean;
  uploader: string;
}

export interface BackendInfo {
  id: string;
  name: string;
  available: boolean;
}

export interface Diagnostics {
  version: string;
  sessionType: string;
  wayland: boolean;
  x11: boolean;
  desktop: string;
  activeBackend: string | null;
  backends: BackendInfo[];
  saveDirectory: string;
  configPath: string;
  historyPath: string;
}
