<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, fileSrc } from '../api';
  import {
    app,
    doCapture,
    doScrollingCapture,
    startRecording,
    stopRecording,
    startAutoCapture,
    stopAutoCapture,
    reloadHistory,
    toast,
  } from '../state.svelte';
  import type { MonitorInfo, WindowInfo } from '../types';
  import HistoryPanel from './HistoryPanel.svelte';
  import SettingsDialog from './SettingsDialog.svelte';

  const captureActions = [
    { kind: 'region' as const, label: 'Region', hint: 'Select an area', icon: '▣', hotkey: 'PrntScr' },
    { kind: 'fullscreen' as const, label: 'Fullscreen', hint: 'Entire screen', icon: '⛶', hotkey: 'Alt+PrntScr' },
    { kind: 'window' as const, label: 'Window', hint: 'Pick a window', icon: '❐', hotkey: 'Ctrl+PrntScr' },
    { kind: 'scrolling' as const, label: 'Scrolling', hint: 'Stitch a long page', icon: '⇕', hotkey: 'Shift+PrntScr' },
  ] as const;

  const recordActions = [
    { kind: 'region' as const, label: 'Region', hint: 'Select an area', icon: '▣' },
    { kind: 'fullscreen' as const, label: 'Fullscreen', hint: 'Entire screen', icon: '⛶' },
    { kind: 'window' as const, label: 'Window', hint: 'Focused window', icon: '❐' },
  ] as const;

  const delays = [0, 1, 3, 5, 10];

  let monitors = $state<MonitorInfo[]>([]);
  let windows = $state<WindowInfo[]>([]);
  let loadingWindows = $state(false);
  let showWindows = $state(false);
  let showHistory = $state(false);
  let showSettings = $state(false);
  let showDelayMenu = $state(false);
  let showMonitorMenu = $state(false);

  let delay = $state<number>(0);
  let monitor = $state<string>('');

  function formatElapsed(start: number): string {
    const total = Math.floor((Date.now() - start) / 1000);
    const mins = Math.floor(total / 60);
    const secs = total % 60;
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }

  function startAuto() {
    void startAutoCapture('region', {
      intervalMs: 3000,
      changeDetection: false,
      maxCount: 0,
      maxDurationSeconds: 0,
      monitor: monitor || undefined,
    });
  }

  async function hideToolbar() {
    try {
      await getCurrentWindow().hide();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  onMount(async () => {
    try {
      monitors = await api.listMonitors();
    } catch {
      monitors = [];
    }
    // Sync from config
    delay = app.config?.captureDelaySeconds ?? 0;
    monitor = app.config?.monitor ?? '';
  });

  async function loadWindows() {
    if (loadingWindows) return;
    loadingWindows = true;
    try {
      windows = await api.listWindows();
      if (windows.length === 0) toast('No windows found', 'info');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      loadingWindows = false;
    }
  }

  function chooseWindow(windowInfo: WindowInfo) {
    showWindows = false;
    void doCapture('window', { windowId: windowInfo.id, delaySeconds: delay, monitor: monitor || undefined });
  }

  function setDelay(seconds: number) {
    delay = seconds;
    showDelayMenu = false;
  }

  function setMonitor(name: string) {
    monitor = name;
    showMonitorMenu = false;
  }

  async function openLast() {
    if (app.last?.path) {
      try {
        await api.openEditor(app.last.path);
      } catch (err) {
        toast(String(err), 'error');
      }
    }
  }

  async function openLastRecording() {
    if (app.lastRecording?.path) {
      try {
        await api.openPath(app.lastRecording.path);
      } catch (err) {
        toast(String(err), 'error');
      }
    }
  }
</script>

<div class="toolbar-window">
  <div class="toolbar" role="toolbar" aria-label="ShareV capture toolbar">
    <!-- Capture section -->
    <div class="section capture-section" data-section="capture">
      <div class="section-label">Capture</div>
      <div class="button-group">
        {#each captureActions as action (action.kind)}
          {#if action.kind === 'window'}
            <button
              class="tool-btn"
              class:active={app.busy && app.busyKind === 'window'}
              disabled={app.busy}
              onclick={showWindows ? undefined : () => { showWindows = true; void loadWindows(); }}
              title={`${action.label} (${action.hotkey})`}
            >
              <span class="btn-icon">{action.icon}</span>
              <span class="btn-label">{action.label}</span>
              <span class="btn-hotkey">{action.hotkey}</span>
              {#if app.busy && app.busyKind === 'window'}
                <span class="spinner" aria-busy="true"></span>
              {/if}
            </button>
          {:else}
            <button
              class="tool-btn"
              class:active={app.busy && app.busyKind === action.kind}
              disabled={app.busy}
              onclick={() =>
                action.kind === 'scrolling'
                  ? doScrollingCapture({ delaySeconds: delay, monitor: monitor || undefined })
                  : doCapture(action.kind, { delaySeconds: delay, monitor: monitor || undefined })
              }
              title={`${action.label} (${action.hotkey})`}
            >
              <span class="btn-icon">{action.icon}</span>
              <span class="btn-label">{action.label}</span>
              <span class="btn-hotkey">{action.hotkey}</span>
              {#if app.busy && app.busyKind === action.kind}
                <span class="spinner" aria-busy="true"></span>
              {/if}
            </button>
          {/if}
        {/each}
      </div>
    </div>

    <div class="divider" aria-hidden="true"></div>

    <!-- Record section -->
    <div class="section record-section" data-section="record">
      <div class="section-label">Record</div>
      {#if app.recording}
        <div class="recording-active">
          <span class="rec-dot" aria-live="polite"></span>
          <span class="rec-label">Recording {app.recording.kind}</span>
          <span class="rec-format">{app.recording.format.toUpperCase()}</span>
          <span class="rec-time" aria-live="polite">{app.recordingStartedAt ? formatElapsed(app.recordingStartedAt) : '00:00'}</span>
          <button class="tool-btn stop-btn" onclick={stopRecording} title="Stop recording (Ctrl+Shift+R)">
            <span class="btn-icon">■</span>
            <span class="btn-label">Stop</span>
          </button>
        </div>
      {:else}
        <div class="button-group">
          {#each recordActions as action (action.kind)}
            <button
              class="tool-btn"
              onclick={() => startRecording(action.kind, { delaySeconds: delay, monitor: monitor || undefined })}
              title={action.label}
            >
              <span class="btn-icon">{action.icon}</span>
              <span class="btn-label">{action.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="divider" aria-hidden="true"></div>

    <!-- Options: Delay & Monitor (shared for capture/record) -->
    <div class="section options-section">
      <div class="option-group">
        <button
          class="option-btn"
          class:expanded={showDelayMenu}
          onclick={() => { showDelayMenu = !showDelayMenu; showMonitorMenu = false; }}
          aria-haspopup="menu"
          aria-expanded={showDelayMenu}
        >
          <span class="opt-icon">⏱</span>
          <span class="opt-value">{delay === 0 ? 'No delay' : `${delay}s`}</span>
          <span class="opt-chevron">{showDelayMenu ? '▲' : '▼'}</span>
        </button>
        {#if showDelayMenu}
          <div class="option-menu" role="menu">
            {#each delays as d (d)}
              <button role="menuitem" class="menu-item" class:active={delay === d} onclick={() => setDelay(d)}>
                {d === 0 ? 'No delay' : `${d}s`}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="option-group">
        <button
          class="option-btn"
          class:expanded={showMonitorMenu}
          onclick={() => { showMonitorMenu = !showMonitorMenu; showDelayMenu = false; }}
          aria-haspopup="menu"
          aria-expanded={showMonitorMenu}
        >
          <span class="opt-icon">🖥</span>
          <span class="opt-value">{monitor ? monitor : 'Auto'}</span>
          <span class="opt-chevron">{showMonitorMenu ? '▲' : '▼'}</span>
        </button>
        {#if showMonitorMenu}
          <div class="option-menu" role="menu">
            <button role="menuitem" class="menu-item" class:active={!monitor} onclick={() => setMonitor('')}>
              Auto (focused)
            </button>
            {#each monitors as m (m.name)}
              <button role="menuitem" class="menu-item" class:active={monitor === m.name} onclick={() => setMonitor(m.name)}>
                {m.focused ? '● ' : ''}{m.name} · {m.width}×{m.height}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="divider" aria-hidden="true"></div>

    <!-- Auto capture -->
    <div class="section auto-section">
      <div class="section-label">Auto</div>
      {#if app.autoCapture}
        <div class="auto-running">
          <span class="auto-status">{app.autoCapture.kind} · every {app.autoCapture.intervalMs / 1000}s</span>
          <button class="tool-btn small" onclick={stopAutoCapture}>
            <span class="btn-icon">■</span>
            <span class="btn-label">Stop</span>
          </button>
        </div>
      {:else}
        <button class="tool-btn" onclick={startAuto} title="Start auto capture">
          <span class="btn-icon">⟳</span>
          <span class="btn-label">Auto</span>
        </button>
      {/if}
    </div>

    <div class="divider" aria-hidden="true"></div>

    <!-- History panel toggle -->
    <div class="section history-section">
      <button
        class="tool-btn"
        class:active={showHistory}
        onclick={() => { showHistory = !showHistory; if (showHistory) reloadHistory(); }}
        title="History (Ctrl+H)"
      >
        <span class="btn-icon">☰</span>
        <span class="btn-label">History</span>
        {#if app.history.length > 0}
          <span class="badge">{app.history.length}</span>
        {/if}
      </button>
    </div>

    <!-- Settings -->
    <div class="section settings-section">
      <button
        class="tool-btn"
        class:active={showSettings}
        onclick={() => showSettings = !showSettings}
        title="Settings (Ctrl+,)"
      >
        <span class="btn-icon">⚙</span>
        <span class="btn-label">Settings</span>
      </button>
    </div>

    <!-- Spacer pushes last items right -->
    <div class="spacer" aria-hidden="true"></div>

    <!-- Last capture quick actions -->
    {#if app.last}
      <div class="section last-section">
        <button class="tool-btn last-preview" onclick={openLast} title="Open last capture in editor">
          <img class="thumb" src={fileSrc(app.last.path)} alt={app.last.fileName} />
          <span class="last-name">{app.last.fileName}</span>
        </button>
      </div>
    {/if}

    {#if app.lastRecording}
      <div class="section last-rec-section">
        <button class="tool-btn last-preview" onclick={openLastRecording} title="Open last recording">
          <span class="thumb rec-thumb">{app.lastRecording.format.toUpperCase()}</span>
          <span class="last-name">{app.lastRecording.fileName}</span>
        </button>
      </div>
    {/if}

    <!-- Close/hide -->
    <div class="section close-section">
      <button class="tool-btn ghost" onclick={hideToolbar} title="Hide to tray">
        <span class="btn-icon">✕</span>
      </button>
    </div>
  </div>

  <!-- Window picker overlay -->
  {#if showWindows}
    <div
      class="overlay"
      onclick={() => showWindows = false}
      onkeydown={(e) => e.key === 'Escape' && (showWindows = false)}
      role="button"
      tabindex="0"
      aria-label="Close window picker"
    >
      <div
        class="window-picker"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Capture a window"
        tabindex="-1"
      >
        <div class="picker-head">
          <h2>Capture a window</h2>
          <button class="ghost" onclick={() => showWindows = false}>Close</button>
        </div>
        {#if windows.length === 0}
          <div class="empty">{loadingWindows ? 'Loading windows…' : 'No windows found.'}</div>
        {:else}
          <ul class="window-list">
            {#each windows as w (w.id)}
              <li>
                <button class="window-item" onclick={() => chooseWindow(w)}>
                  <span class="window-title">{w.title || '(untitled)'}</span>
                  <span class="window-app">{w.app || '—'} · {w.width}×{w.height}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {/if}

  <!-- History panel (slides from right) -->
  {#if showHistory}
    <HistoryPanel onClose={() => showHistory = false} />
  {/if}

  <!-- Settings dialog -->
  {#if showSettings}
    <SettingsDialog onClose={() => showSettings = false} />
  {/if}
</div>

<style>
  .toolbar-window {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10000;
    font-family: system-ui, -apple-system, sans-serif;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
    max-width: 100vw;
    overflow-x: auto;
  }

  .section {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .section-label {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    padding: 0 4px;
    white-space: nowrap;
  }

  .button-group {
    display: flex;
    gap: 2px;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.1s ease;
    white-space: nowrap;
    min-height: 36px;
  }

  .tool-btn:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--border);
  }

  .tool-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .tool-btn.active {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }

  .tool-btn.small {
    padding: 4px 8px;
    font-size: 0.7rem;
  }

  .tool-btn.ghost {
    background: transparent;
  }

  .tool-btn.ghost:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .stop-btn {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }

  .stop-btn:hover:not(:disabled) {
    background: var(--danger-2);
    border-color: var(--danger-2);
  }

  .btn-icon {
    font-size: 1.1rem;
    line-height: 1;
  }

  .btn-label {
    display: none;
  }

  .btn-hotkey {
    display: none;
    font-size: 0.6rem;
    color: var(--text-dim);
    padding: 1px 4px;
    background: var(--surface-3);
    border-radius: 3px;
  }

  @media (min-width: 800px) {
    .btn-label {
      display: inline;
    }
    .btn-hotkey {
      display: inline;
    }
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .divider {
    width: 1px;
    height: 28px;
    background: var(--border);
    margin: 0 4px;
  }

  /* Recording active state */
  .recording-active {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border: 1px solid var(--danger);
    border-radius: 8px;
  }

  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .rec-label {
    font-size: 0.78rem;
    font-weight: 500;
  }

  .rec-format {
    font-size: 0.65rem;
    padding: 1px 5px;
    background: var(--danger);
    color: #fff;
    border-radius: 3px;
    font-weight: 600;
  }

  .rec-time {
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
    font-weight: 600;
    font-family: ui-monospace, monospace;
  }

  /* Options */
  .option-group {
    position: relative;
  }

  .option-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .option-btn:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--border);
  }

  .option-btn.expanded {
    background: var(--surface-2);
    border-color: var(--accent);
  }

  .opt-icon {
    font-size: 1rem;
  }

  .opt-value {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .opt-chevron {
    font-size: 0.6rem;
    color: var(--text-dim);
    transition: transform 0.1s ease;
  }

  .option-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 160px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    padding: 4px;
    z-index: 100;
  }

  .menu-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 6px 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
  }

  .menu-item:hover {
    background: var(--surface-2);
  }

  .menu-item.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
    font-weight: 500;
  }

  /* Auto section */
  .auto-running {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .auto-status {
    font-size: 0.72rem;
    color: var(--text-dim);
  }

  /* History section */
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 16px;
    height: 16px;
    padding: 0 5px;
    background: var(--accent);
    color: #fff;
    font-size: 0.65rem;
    font-weight: 600;
    border-radius: 8px;
  }

  /* Last capture preview */
  .last-preview {
    gap: 8px;
    padding: 4px 8px;
  }

  .thumb {
    width: 36px;
    height: 24px;
    object-fit: cover;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--surface-3);
  }

  .rec-thumb {
    width: 36px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.55rem;
    font-weight: 700;
    border-radius: 4px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    color: var(--accent);
  }

  .last-name {
    font-size: 0.7rem;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Overlay */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .window-picker {
    width: 90%;
    max-width: 700px;
    max-height: 80vh;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .picker-head h2 {
    margin: 0;
    font-size: 0.95rem;
  }

  .picker-head button {
    padding: 4px 10px;
    font-size: 0.78rem;
  }

  .window-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.5rem;
  }

  .window-item {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.7rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    cursor: pointer;
  }

  .window-item:hover {
    border-color: var(--accent);
  }

  .window-title {
    font-weight: 600;
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .window-app {
    font-size: 0.7rem;
    color: var(--text-dim);
  }

  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--text-dim);
  }

  /* Spacer */
  .spacer {
    flex: 1;
  }

  /* History panel & Settings dialog overlay */
  .panel-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 2000;
  }

  .panel {
    position: fixed;
    z-index: 2001;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.5);
  }
</style>