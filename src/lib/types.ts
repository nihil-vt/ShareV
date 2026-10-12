export type ImageFormat = 'png' | 'jpeg' | 'webp';
export type CaptureKind = 'region' | 'fullscreen' | 'window' | 'scrolling';
export type UploaderKind = 'none' | 'imgur' | 'http' | 'command';
export type CopyFormat = 'url' | 'markdown' | 'html';
export type RecordFormat = 'mp4' | 'webm' | 'gif';
export type AudioMode = 'none' | 'system' | 'microphone' | 'both';
export type HotkeyBackend = 'auto' | 'portal' | 'global-shortcut' | 'none';

/** A single post-capture / post-upload action id. */
export type CaptureTask =
  | 'save'
  | 'history'
  | 'copy'
  | 'editor'
  | 'open'
  | 'upload'
  | 'pin'
  | 'command'
  | 'notify'
  | 'copyLink'
  | 'ocr';

export interface Geometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface HotkeyBinding {
  action: string;
  accelerator: string;
}

export interface Hotkeys {
  enabled: boolean;
  backend: HotkeyBackend;
  /** Ordered action bindings; empty means "derive from the legacy keys". */
  bindings: HotkeyBinding[];
  region: string;
  fullscreen: string;
  window: string;
}

export interface AfterCaptureConfig {
  tasks?: CaptureTask[] | null;
  command: string;
}

export interface AfterUploadConfig {
  tasks?: CaptureTask[] | null;
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

export interface ScrollConfig {
  maxFrames: number;
  frameDelayMs: number;
  similarityThreshold: number;
}

export interface AutoCaptureConfig {
  intervalMs: number;
  changeDetection: boolean;
  changeThreshold: number;
  maxCount: number;
  maxDurationSeconds: number;
}

export interface AutoCaptureInfo {
  kind: string;
  intervalMs: number;
  changeDetection: boolean;
  startedAt: string;
  count: number;
  maxCount: number;
  maxDurationSeconds: number;
}

export type EffectKind =
  | 'none'
  | 'shadow'
  | 'border'
  | 'watermark'
  | 'canvasResize'
  | 'blur'
  | 'roundedCorners';

export interface ShadowParams {
  offsetX: number;
  offsetY: number;
  blurRadius: number;
  color: string;
  opacity: number;
}

export interface BorderParams {
  width: number;
  color: string;
}

export interface WatermarkParams {
  text: string;
  fontSize: number;
  color: string;
  opacity: number;
  position: string;
}

export interface CanvasResizeParams {
  padding: number;
  color: string;
}

export interface BlurParams {
  radius: number;
}

export interface RoundedCornersParams {
  radius: number;
}

export interface EffectsConfig {
  effects: EffectKind[];
  shadow: ShadowParams;
  border: BorderParams;
  watermark: WatermarkParams;
  canvasResize: CanvasResizeParams;
  blur: BlurParams;
  roundedCorners: RoundedCornersParams;
}

export interface FeatureFlags {
  /** OCR via Tesseract. Off means Tesseract never needs to be installed. */
  ocr: boolean;
}

export interface Config {
  saveDirectory: string;
  imageFormat: ImageFormat;
  jpegQuality: number;
  filenameTemplate: string;
  captureDelaySeconds: number;
  monitor: string;
  copyToClipboard: boolean;
  openEditorAfterCapture: boolean;
  showNotification: boolean;
  maxHistory: number;
  captureBackend: string;
  hotkeys: Hotkeys;
  upload: UploadConfig;
  record: RecordConfig;
  afterCapture: AfterCaptureConfig;
  afterUpload: AfterUploadConfig;
  /** Tesseract language code used by the `ocr` task (e.g. "eng"). */
  ocrLanguage: string;
  /** Toggleable features with optional system dependencies. */
  features: FeatureFlags;
  scroll: ScrollConfig;
  effects: EffectsConfig;
  auto: AutoCaptureConfig;
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
  /** URL returned by the `upload` task, when it ran. */
  url?: string | null;
  /** Whether the annotation editor should open for this capture. */
  openEditor: boolean;
}

export interface WindowInfo {
  /** Backend-specific window id (Hyprland address, Sway node id, X11 window id). */
  id: string;
  title: string;
  app: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorInfo {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  focused: boolean;
}

export interface PinInfo {
  path: string;
  width: number;
  height: number;
}

export interface EditorInfo {
  path: string;
  width: number;
  height: number;
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
  /** Active hotkey mechanism: "portal", "global-shortcut" or "none". */
  hotkeyBackend: string;
  /** Whether the XDG portal GlobalShortcuts interface is reachable. */
  portalShortcuts: boolean;
}
