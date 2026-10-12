<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Sidebar from './lib/components/Sidebar.svelte';
  import CaptureView from './lib/components/CaptureView.svelte';
  import RecordView from './lib/components/RecordView.svelte';
  import HistoryView from './lib/components/HistoryView.svelte';
  import SettingsView from './lib/components/SettingsView.svelte';
  import DiagnosticsView from './lib/components/DiagnosticsView.svelte';
  import EditorWindow from './lib/components/EditorWindow.svelte';
  import PinView from './lib/components/PinView.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import { api } from './lib/api';
  import { app, init, reloadHistory, toast } from './lib/state.svelte';
  import type { CaptureResult, RecordingInfo, RecordingResult } from './lib/types';

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
      toast(`Saved ${event.payload.fileName}`, 'success');
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

    // The `open-editor` hotkey action opens the editor in its own window.
    const unlistenEditor = listen('open-editor', () => {
      if (app.last?.path) {
        api.openEditor(app.last.path).catch((err) => toast(`Could not open editor: ${err}`, 'error'));
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenStart.then((fn) => fn());
      unlistenStop.then((fn) => fn());
      unlistenEditor.then((fn) => fn());
    };
  });
</script>

{#if pinWindow}
  <PinView />
{:else if editorWindow}
  <EditorWindow />
{:else}
  <div class="app">
    <Sidebar />
    <main class="content">
      {#if !app.ready}
        <div class="loading">Loading…</div>
      {:else if app.view === 'capture'}
        <CaptureView />
      {:else if app.view === 'record'}
        <RecordView />
      {:else if app.view === 'history'}
        <HistoryView />
      {:else if app.view === 'settings'}
        <SettingsView />
      {:else}
        <DiagnosticsView />
      {/if}
    </main>
    <Toasts />
  </div>
{/if}
