import { api, type AutoCaptureOptions, type CaptureOptions } from './api';
import type {
  AudioDevice,
  AutoCaptureInfo,
  CaptureKind,
  CaptureResult,
  Config,
  Diagnostics,
  HistoryEntry,
  RecordingInfo,
  RecordingResult,
  UploadOutcome,
} from './types';

export type View = 'capture' | 'record' | 'history' | 'settings' | 'diagnostics' | 'editor';

export interface Toast {
  id: number;
  text: string;
  kind: 'info' | 'success' | 'error';
}

let toastSeq = 0;

export const app = $state({
  view: 'capture' as View,
  ready: false,
  busy: false,
  busyKind: null as CaptureKind | null,
  uploading: false,
  config: null as Config | null,
  history: [] as HistoryEntry[],
  last: null as CaptureResult | null,
  lastUpload: null as UploadOutcome | null,
  editing: null as HistoryEntry | null,
  diagnostics: null as Diagnostics | null,
  toasts: [] as Toast[],
  // Recording
  recording: null as RecordingInfo | null,
  recordingStartedAt: null as number | null,
  lastRecording: null as RecordingResult | null,
  audioDevices: [] as AudioDevice[],
  // Auto capture
  autoCapture: null as AutoCaptureInfo | null,
});

export function toast(text: string, kind: Toast['kind'] = 'info') {
  const id = ++toastSeq;
  app.toasts.push({ id, text, kind });
  setTimeout(() => {
    app.toasts = app.toasts.filter((t) => t.id !== id);
  }, 4200);
}

export async function init() {
  try {
    app.config = await api.getConfig();
    app.history = await api.listHistory();
    app.ready = true;
    refreshRecordingStatus();
    refreshAutoCapture();
  } catch (err) {
    toast(`Failed to start: ${err}`, 'error');
    app.ready = true;
  }
}

export async function reloadHistory() {
  try {
    app.history = await api.listHistory();
  } catch (err) {
    toast(String(err), 'error');
  }
}

/**
 * Upload a saved capture to the configured destination and copy the link.
 * Returns the outcome on success, or null if the upload failed.
 */
export async function uploadFile(path: string): Promise<UploadOutcome | null> {
  if (app.uploading) return null;
  app.uploading = true;
  try {
    const outcome = await api.uploadPath(path);
    app.lastUpload = outcome;
    await reloadHistory();
    if (outcome.copied) {
      toast(`Uploaded via ${outcome.uploader} — link copied`, 'success');
    } else {
      toast(`Uploaded via ${outcome.uploader}`, 'success');
    }
    return outcome;
  } catch (err) {
    toast(`Upload failed: ${err}`, 'error');
    return null;
  } finally {
    app.uploading = false;
  }
}

export async function doCapture(kind: CaptureKind, options: CaptureOptions = {}) {
  if (app.busy) return;
  app.busy = true;
  app.busyKind = kind;
  try {
    // The backend emits `capture-completed`, which the app-level listener turns
    // into the "Last capture" update and a toast — don't also handle it here.
    await api.capture(kind, options);
  } catch (err) {
    reportCaptureError(err);
  } finally {
    app.busy = false;
    app.busyKind = null;
  }
}

/** Capture a scrolling region and stitch it into one tall image. */
export async function doScrollingCapture(options: CaptureOptions = {}) {
  if (app.busy) return;
  app.busy = true;
  app.busyKind = 'scrolling';
  try {
    await api.captureScrolling(options.delaySeconds);
  } catch (err) {
    reportCaptureError(err);
  } finally {
    app.busy = false;
    app.busyKind = null;
  }
}

function reportCaptureError(err: unknown) {
  const message = String(err);
  if (message.toLowerCase().includes('cancel')) {
    toast('Capture cancelled', 'info');
  } else {
    toast(`Capture failed: ${message}`, 'error');
  }
}

export async function saveConfig(next: Config) {
  try {
    await api.saveConfig($state.snapshot(next) as Config);
    app.config = next;
    toast('Settings saved', 'success');
  } catch (err) {
    toast(`Could not save settings: ${err}`, 'error');
  }
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

/** Sync the in-progress recording state with the backend. */
export async function refreshRecordingStatus() {
  try {
    const info = await api.recordingStatus();
    app.recording = info;
    app.recordingStartedAt = info ? Date.now() : null;
  } catch {
    // The backend may not be ready yet; ignore.
  }
}

/** Start recording a region, the full screen or a window. */
export async function startRecording(kind: CaptureKind, options: CaptureOptions = {}) {
  if (app.recording) return;
  try {
    const info = await api.startRecording(kind, options);
    app.recording = info;
    app.recordingStartedAt = Date.now();
    app.lastRecording = null;
    toast(`Recording ${kind}…`, 'success');
  } catch (err) {
    const message = String(err);
    if (message.toLowerCase().includes('cancel')) {
      toast('Recording cancelled', 'info');
    } else {
      toast(`Could not start recording: ${message}`, 'error');
    }
  }
}

/** Stop the in-progress recording and wait for the file to be finalized. */
export async function stopRecording(): Promise<RecordingResult | null> {
  if (!app.recording) return null;
  try {
    const result = await api.stopRecording();
    app.lastRecording = result;
    app.recording = null;
    app.recordingStartedAt = null;
    await reloadHistory();
    toast(`Saved ${result.fileName}`, 'success');
    return result;
  } catch (err) {
    app.recording = null;
    app.recordingStartedAt = null;
    toast(`Recording failed: ${err}`, 'error');
    return null;
  }
}

/** Load PulseAudio/PipeWire source devices for the settings pickers. */
export async function loadAudioDevices() {
  try {
    app.audioDevices = await api.listAudioDevices();
  } catch {
    app.audioDevices = [];
  }
}

// ---------------------------------------------------------------------------
// Auto capture
// ---------------------------------------------------------------------------

/** Sync the active auto-capture session with the backend. */
export async function refreshAutoCapture() {
  try {
    app.autoCapture = await api.autoCaptureStatus();
  } catch {
    // The backend may not be ready yet; ignore.
  }
}

/** Start a timed auto-capture session over a region, the screen or a window. */
export async function startAutoCapture(
  kind: CaptureKind,
  options: AutoCaptureOptions = {},
) {
  if (app.autoCapture) return;
  try {
    app.autoCapture = await api.startAutoCapture(kind, options);
    toast(`Auto capture started (${kind})`, 'success');
  } catch (err) {
    const message = String(err);
    if (message.toLowerCase().includes('cancel')) {
      toast('Auto capture cancelled', 'info');
    } else {
      toast(`Could not start auto capture: ${message}`, 'error');
    }
  }
}

/** Ask the active auto-capture session to stop. */
export async function stopAutoCapture() {
  if (!app.autoCapture) return;
  try {
    await api.stopAutoCapture();
    // The loop clears the state and emits "auto-capture-stopped"; refresh too
    // in case this was called from a component that misses the event.
    await refreshAutoCapture();
  } catch (err) {
    toast(`Could not stop auto capture: ${err}`, 'error');
  }
}

/** Pin the most recent capture to the screen (returns the pin window label). */
export async function pinLastCapture(): Promise<string | null> {
  const label = await api.pinLast();
  if (label) toast('Pinned to screen — double-click it to close', 'success');
  return label;
}

/** Upload the most recent capture (used by the tray / "Upload last" action). */
export async function uploadLastCapture(): Promise<UploadOutcome | null> {
  const outcome = await api.uploadLast();
  if (outcome) toast(`Uploaded via ${outcome.uploader}`, 'success');
  return outcome;
}
