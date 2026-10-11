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
  import EditorView from './lib/components/EditorView.svelte';
  import PinView from './lib/components/PinView.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import { app, init, reloadHistory, toast } from './lib/state.svelte';
  import type { CaptureResult, RecordingInfo, RecordingResult } from './lib/types';

  // Pin windows are separate webviews created by the backend; they render a
  // single image instead of the full app.
  const pinWindow = $derived(getCurrentWindow().label.startsWith('pin-'));

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

    // The `open-editor` hotkey action surfaces the main window.
    const unlistenEditor = listen('open-editor', () => {
      if (app.view !== 'editor') app.view = 'capture';
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
{:else}
  <div class="app">
    <Sidebar />
    <main class="content">
      {#if !app.ready}
        <div class="loading">Loading…</div>
      {:else if app.view === 'editor'}
        <EditorView />
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
