import { getVersion } from '@tauri-apps/api/app';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { api } from './api';

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'uptodate'
  | 'downloading'
  | 'ready'
  | 'error';

export const updates = $state({
  current: '',
  flatpak: false,
  checking: false,
  status: 'idle' as UpdateStatus,
  version: null as string | null,
  notes: null as string | null,
  downloaded: 0,
  total: 0,
  error: null as string | null,
});

let pending: Update | null = null;

/** Load the running version and detect a sandboxed (Flatpak) install. */
export async function initUpdates() {
  try {
    updates.current = await getVersion();
  } catch {
    updates.current = '';
  }
  try {
    updates.flatpak = await api.isFlatpak();
  } catch {
    updates.flatpak = false;
  }
}

/** Ask the configured endpoint whether a newer release exists. */
export async function checkForUpdates() {
  if (updates.flatpak || updates.checking) return;
  // Release any update handle we are about to replace.
  if (pending) {
    try {
      await pending.close();
    } catch {
      /* ignore */
    }
    pending = null;
  }
  updates.checking = true;
  updates.status = 'checking';
  updates.error = null;
  try {
    pending = await check();
    if (pending) {
      updates.version = pending.version;
      updates.notes = pending.body ?? null;
      updates.status = 'available';
    } else {
      pending = null;
      updates.status = 'uptodate';
    }
  } catch (err) {
    updates.error = String(err);
    updates.status = 'error';
  } finally {
    updates.checking = false;
  }
}

/** Download and install the pending update, then relaunch. */
export async function installUpdate() {
  if (!pending) return;
  updates.status = 'downloading';
  updates.downloaded = 0;
  updates.total = 0;
  updates.error = null;
  try {
    await pending.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        updates.total = event.data.contentLength ?? 0;
      } else if (event.event === 'Progress') {
        updates.downloaded += event.data.chunkLength;
      } else if (event.event === 'Finished') {
        updates.status = 'ready';
      }
    });
    updates.status = 'ready';
    await relaunch();
  } catch (err) {
    updates.error = String(err);
    updates.status = 'error';
  }
}

export function updateProgress(): number {
  if (updates.total <= 0) return 0;
  return Math.min(100, Math.round((updates.downloaded / updates.total) * 100));
}
