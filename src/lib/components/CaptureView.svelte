<script lang="ts">
  import { onMount } from 'svelte';
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import { app, doCapture, toast, uploadFile } from '../state.svelte';
  import type { CaptureKind, MonitorInfo, WindowInfo } from '../types';

  const actions = [
    { kind: 'region' as const, label: 'Region', hint: 'Select an area', icon: '▣' },
    { kind: 'fullscreen' as const, label: 'Fullscreen', hint: 'Entire screen', icon: '⛶' },
    { kind: 'window' as const, label: 'Window', hint: 'Pick a window', icon: '❐' },
  ];

  const delays = [0, 1, 3, 5, 10];

  let monitors = $state<MonitorInfo[]>([]);
  let windows = $state<WindowInfo[]>([]);
  let loadingWindows = $state(false);
  let showWindows = $state(false);
  let selectedWindow = $state<WindowInfo | null>(null);

  let delay = $state<number>(app.config?.captureDelaySeconds ?? 0);
  let monitor = $state<string>(app.config?.monitor ?? '');

  onMount(async () => {
    try {
      monitors = await api.listMonitors();
    } catch {
      monitors = [];
    }
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

  function toggleWindows() {
    showWindows = !showWindows;
    if (showWindows && windows.length === 0) void loadWindows();
  }

  function chooseWindow(windowInfo: WindowInfo) {
    selectedWindow = windowInfo;
    showWindows = false;
    void doCapture('window', { windowId: windowInfo.id });
  }

  async function openPath(path: string) {
    try {
      await api.openPath(path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function revealPath(path: string) {
    try {
      await api.revealPath(path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function copyClip(path: string) {
    try {
      await api.copyFileToClipboard(path);
      toast('Copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function copyLink(url: string) {
    try {
      await api.copyText(url);
      toast('Link copied', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

<section class="view">
  <header class="view-head">
    <h1>Capture</h1>
    <p class="sub">Grab the screen, save it, and copy it to the clipboard.</p>
  </header>

  <div class="capture-bar">
    <label class="opt">
      <span>Delay</span>
      <select bind:value={delay}>
        {#each delays as d (d)}
          <option value={d}>{d === 0 ? 'None' : `${d}s`}</option>
        {/each}
      </select>
    </label>
    <label class="opt grow">
      <span>Monitor</span>
      <select bind:value={monitor}>
        <option value="">Focused / default</option>
        {#each monitors as m (m.name)}
          <option value={m.name}>{m.focused ? '● ' : ''}{m.name} · {m.width}×{m.height}</option>
        {/each}
      </select>
    </label>
  </div>

  <div class="capture-grid">
    {#each actions as action (action.kind)}
      {#if action.kind === 'window'}
        <button class="capture-card" disabled={app.busy} onclick={toggleWindows}>
          <span class="card-icon">{action.icon}</span>
          <span class="card-label">{action.label}</span>
          <span class="card-hint">{selectedWindow ? selectedWindow.title : action.hint}</span>
          {#if app.busy && app.busyKind === action.kind}
            <span class="card-spinner">working…</span>
          {/if}
        </button>
      {:else}
        <button
          class="capture-card"
          disabled={app.busy}
          onclick={() =>
            doCapture(action.kind, {
              delaySeconds: delay,
              monitor: monitor || undefined,
            })}
        >
          <span class="card-icon">{action.icon}</span>
          <span class="card-label">{action.label}</span>
          <span class="card-hint">{action.hint}</span>
          {#if app.busy && app.busyKind === action.kind}
            <span class="card-spinner">working…</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>

  {#if showWindows}
    <div class="window-picker">
      <div class="window-picker-head">
        <h2>Capture a window</h2>
        <button class="ghost" onclick={toggleWindows}>Close</button>
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
  {/if}

  {#if app.last}
    <div class="last">
      <div class="last-head">
        <h2>Last capture</h2>
        <span class="pill">{app.last.copied ? 'copied to clipboard' : 'saved'}</span>
      </div>
      <div class="last-body">
        <img src={fileSrc(app.last.path)} alt={app.last.fileName} />
        <div class="last-meta">
          <div class="file">{app.last.fileName}</div>
          <dl>
            <div><dt>Size</dt><dd>{app.last.width} × {app.last.height}</dd></div>
            <div><dt>Bytes</dt><dd>{formatBytes(app.last.bytes)}</dd></div>
            <div><dt>Backend</dt><dd>{app.last.backend}</dd></div>
            <div><dt>When</dt><dd>{formatDate(app.last.createdAt)}</dd></div>
          </dl>
          <div class="last-actions">
            <button onclick={() => openPath(app.last!.path)}>Open</button>
            <button onclick={() => revealPath(app.last!.path)}>Folder</button>
            <button onclick={() => copyClip(app.last!.path)}>Copy</button>
            <button
              class="primary"
              onclick={() => uploadFile(app.last!.path)}
              disabled={app.uploading || app.config?.upload.uploader === 'none'}
            >
              {app.uploading ? 'Uploading…' : 'Upload'}
            </button>
          </div>
          {#if app.lastUpload}
            <div class="upload-link">
              <span class="dim">Uploaded via {app.lastUpload.uploader}</span>
              <code class="url">{app.lastUpload.url}</code>
              <button class="ghost" onclick={() => openPath(app.lastUpload!.url)}>Open link</button>
              <button class="ghost" onclick={() => copyLink(app.lastUpload!.link)}>Copy link</button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .capture-bar {
    display: flex;
    gap: 1rem;
    align-items: flex-end;
    margin-bottom: 1.1rem;
    padding: 0.8rem 1rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  .opt {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .opt.grow {
    flex: 1;
  }
  .capture-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .capture-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    padding: 1.75rem 1rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    color: var(--text);
    cursor: pointer;
    transition: transform 0.12s ease, border-color 0.12s ease, background 0.12s ease;
  }
  .capture-card:hover:not(:disabled) {
    transform: translateY(-2px);
    border-color: var(--accent);
    background: var(--surface-3);
  }
  .capture-card:disabled {
    opacity: 0.55;
    cursor: progress;
  }
  .card-icon {
    font-size: 2rem;
    color: var(--accent);
  }
  .card-label {
    font-size: 1.05rem;
    font-weight: 600;
  }
  .card-hint {
    font-size: 0.8rem;
    color: var(--text-dim);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .card-spinner {
    font-size: 0.75rem;
    color: var(--accent);
  }
  .window-picker {
    margin-bottom: 1.5rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0.9rem 1rem;
  }
  .window-picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
  }
  .window-picker-head h2 {
    margin: 0;
    font-size: 0.95rem;
  }
  .window-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.5rem;
    max-height: 320px;
    overflow-y: auto;
  }
  .window-item {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    width: 100%;
    text-align: left;
    padding: 0.55rem 0.7rem;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--text);
    cursor: pointer;
  }
  .window-item:hover {
    border-color: var(--accent);
  }
  .window-title {
    font-weight: 600;
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .window-app {
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  .empty {
    padding: 1.5rem;
    text-align: center;
    color: var(--text-dim);
    font-size: 0.85rem;
  }
  .last {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 1rem 1.15rem;
  }
  .last-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.85rem;
  }
  .last-head h2 {
    margin: 0;
    font-size: 1rem;
  }
  .last-body {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(220px, 1fr);
    gap: 1.15rem;
    align-items: start;
  }
  .last-body img {
    width: 100%;
    max-height: 320px;
    object-fit: contain;
    background: var(--surface-3);
    border-radius: 10px;
    border: 1px solid var(--border);
  }
  .last-meta .file {
    font-weight: 600;
    word-break: break-all;
    margin-bottom: 0.6rem;
  }
  dl {
    margin: 0 0 0.9rem;
    display: grid;
    gap: 0.3rem;
  }
  dl > div {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: 0.86rem;
  }
  dt {
    color: var(--text-dim);
  }
  dd {
    margin: 0;
  }
  .last-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .upload-link {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    flex-wrap: wrap;
    margin-top: 0.7rem;
    padding-top: 0.7rem;
    border-top: 1px solid var(--border);
  }
  .upload-link .dim {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .url {
    background: var(--surface-3);
    padding: 0.15rem 0.45rem;
    border-radius: 6px;
    font-size: 0.76rem;
    max-width: 340px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
