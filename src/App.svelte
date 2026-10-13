<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import MainWindow from './lib/components/MainWindow.svelte';
  import EditorWindow from './lib/components/EditorWindow.svelte';
  import PinView from './lib/components/PinView.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import { api } from './lib/api';
  import { app, init, reloadHistory, toast } from './lib/state.svelte';
  import type { AutoCaptureInfo, CaptureResult, RecordingInfo, RecordingResult } from './lib/types';

  // Pin and editor windows are separate webviews created by the backend;
  // they render their own UI instead of the full app.
  const pinWindow = $derived(getCurrentWindow().label.startsWith('pin-'));
  const editorWindow = $derived(getCurrentWindow().label.startsWith('editor-'));

  $effect(() => {
    document.documentElement.classList.toggle('pin-mode', pinWindow);
  });

  onMount(() => {
    if (pinWindow) return;

    init();

    // Captures triggered from the tray or a global hotkey arrive as events.
    const unlisten = listen<CaptureResult>('capture-completed', (event) => {
      app.last = event.payload;
      app.lastUpload = null;
      reloadHistory();
      // Auto capture can save many frames; don't toast for every one.
      if (!app.autoCapture) {
        toast(`Saved ${event.payload.fileName}`, 'success');
      }
    });

    // Recording lifecycle events (also emitted when driven from the tray).
    const unlistenStart = listen<RecordingInfo>('recording-started', (event) => {
      if (!app.recording) {
        app.recording = event.payload;
        app.recordingStartedAt = Date.now();
        app.lastRecording = null;
      }
    });
    const unlistenStop = listen<RecordingResult>('recording-stopped', (event) => {
      app.recording = null;
      app.recordingStartedAt = null;
      app.lastRecording = event.payload;
      reloadHistory();
    });
    const unlistenAbort = listen('recording-aborted', () => {
      app.recording = null;
      app.recordingStartedAt = null;
      toast('Recording discarded', 'info');
    });

    // The `open-editor` hotkey action opens the editor in its own window.
    const unlistenEditor = listen('open-editor', () => {
      if (app.last?.path) {
        api.openEditor(app.last.path).catch((err) => toast(`Could not open editor: ${err}`, 'error'));
      }
    });

    // Watch folders: new images arrive as history entries.
    const unlistenWatchFile = listen<{ dir: string; path: string }>(
      'watch-file',
      (event) => {
        toast(`Watch folder captured ${event.payload.path.split('/').pop()}`, 'success');
        reloadHistory();
      },
    );

    // Auto-capture lifecycle events (also emitted when driven from the CLI).
    const unlistenAutoStart = listen<AutoCaptureInfo>('auto-capture-started', (event) => {
      app.autoCapture = event.payload;
    });
    const unlistenAutoStop = listen<AutoCaptureInfo>('auto-capture-stopped', (event) => {
      app.autoCapture = null;
      toast(`Auto capture stopped after ${event.payload.count} capture(s)`, 'info');
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenStart.then((fn) => fn());
      unlistenStop.then((fn) => fn());
      unlistenAbort.then((fn) => fn());
      unlistenEditor.then((fn) => fn());
      unlistenWatchFile.then((fn) => fn());
      unlistenAutoStart.then((fn) => fn());
      unlistenAutoStop.then((fn) => fn());
    };
  });
</script>

{#if pinWindow}
  <PinView />
{:else if editorWindow}
  <EditorWindow />
  <Toasts />
{:else}
  <div class="app">
    {#if !app.ready}
      <div class="loading">Loading…</div>
    {:else}
      <MainWindow />
    {/if}
    <Toasts />
  </div>
{/if}
