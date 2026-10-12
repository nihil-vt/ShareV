import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import type {
  AudioDevice,
  AutoCaptureInfo,
  CaptureKind,
  CaptureResult,
  Config,
  Diagnostics,
  EditorInfo,
  Geometry,
  HistoryEntry,
  MonitorInfo,
  PinInfo,
  RecordingInfo,
  RecordingResult,
  UploadOutcome,
  WindowInfo,
} from './types';

/** Optional targeting for a capture / recording. */
export interface CaptureOptions {
  geometry?: Geometry;
  monitor?: string;
  windowId?: string;
  delaySeconds?: number;
}

/** Optional overrides for an auto-capture session. */
export interface AutoCaptureOptions {
  intervalMs?: number;
  changeDetection?: boolean;
  changeThreshold?: number;
  maxCount?: number;
  maxDurationSeconds?: number;
  monitor?: string;
  windowId?: string;
}

/** Typed wrappers around the Rust commands. */
export const api = {
  getConfig: () => invoke<Config>('get_config'),
  saveConfig: (config: Config) => invoke<void>('save_config', { config }),

  capture: (kind: CaptureKind, options: CaptureOptions = {}) =>
    invoke<CaptureResult>('capture', {
      kind,
      geometry: options.geometry ?? null,
      monitor: options.monitor ?? null,
      windowId: options.windowId ?? null,
      delaySeconds: options.delaySeconds ?? null,
    }),

  captureScrolling: (delaySeconds?: number) =>
    invoke<CaptureResult>('capture_scrolling', {
      delaySeconds: delaySeconds ?? null,
    }),

  startAutoCapture: (kind: CaptureKind, options: AutoCaptureOptions = {}) =>
    invoke<AutoCaptureInfo>('start_auto_capture', {
      kind,
      intervalMs: options.intervalMs ?? null,
      changeDetection: options.changeDetection ?? null,
      changeThreshold: options.changeThreshold ?? null,
      maxCount: options.maxCount ?? null,
      maxDurationSeconds: options.maxDurationSeconds ?? null,
      monitor: options.monitor ?? null,
      windowId: options.windowId ?? null,
    }),
  stopAutoCapture: () => invoke<AutoCaptureInfo>('stop_auto_capture'),
  autoCaptureStatus: () => invoke<AutoCaptureInfo | null>('auto_capture_status'),

  listWindows: () => invoke<WindowInfo[]>('list_windows'),
  listMonitors: () => invoke<MonitorInfo[]>('list_monitors'),

  pinCapture: (path: string) => invoke<string>('pin_capture', { path }),
  pinInfo: () => invoke<PinInfo | null>('pin_info'),
  pinLast: () => invoke<string | null>('pin_last'),

  diagnostics: () => invoke<Diagnostics>('diagnostics'),

  listHistory: (limit?: number) => invoke<HistoryEntry[]>('list_history', { limit }),
  deleteHistory: (id: string) => invoke<void>('delete_history', { id }),
  clearHistory: () => invoke<void>('clear_history'),

  openPath: (path: string) => invoke<void>('open_path', { path }),
  revealPath: (path: string) => invoke<void>('reveal_path', { path }),
  copyFileToClipboard: (path: string) => invoke<void>('copy_file_to_clipboard', { path }),
  copyText: (text: string) => invoke<void>('copy_text', { text }),

  readImageBase64: (path: string) => invoke<string>('read_image_base64', { path }),
  applyEdits: (path: string, data: string) =>
    invoke<CaptureResult>('apply_edits', { path, data }),
  copyEdits: (data: string) => invoke<void>('copy_edits', { data }),
  openEditor: (path: string) => invoke<string>('open_editor', { path }),
  editorInfo: () => invoke<EditorInfo | null>('editor_info'),
  ocrStatus: () => invoke<{ available: boolean; languages: string[] }>('ocr_status'),
  ocrRecognize: (path: string, language: string) =>
    invoke<string>('ocr_recognize', { path, language }),
  featureStatus: (feature: string) =>
    invoke<{
      id: string;
      label: string;
      binary: string;
      enabled: boolean;
      available: boolean;
      installCommand: string | null;
    }>('feature_status', { feature }),

  uploadPath: (path: string) => invoke<UploadOutcome>('upload_path', { path }),
  uploadLast: () => invoke<UploadOutcome | null>('upload_last'),

  startRecording: (kind: CaptureKind, options: CaptureOptions = {}) =>
    invoke<RecordingInfo>('start_recording', {
      kind,
      delaySeconds: options.delaySeconds ?? null,
      monitor: options.monitor ?? null,
      windowId: options.windowId ?? null,
    }),
  stopRecording: () => invoke<RecordingResult>('stop_recording'),
  recordingStatus: () => invoke<RecordingInfo | null>('recording_status'),
  listAudioDevices: () => invoke<AudioDevice[]>('list_audio_devices'),

  isFlatpak: () => invoke<boolean>('is_flatpak'),

  pickDirectory: () => invoke<string | null>('pick_directory'),
};

/** Turn an absolute filesystem path into a URL the webview can display. */
export function fileSrc(path: string): string {
  return convertFileSrc(path);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
}

export function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString();
}
