<script lang="ts">
  import { onMount } from 'svelte';
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import { app, startRecording, stopRecording, toast, uploadFile } from '../state.svelte';
  import type { CaptureKind, MonitorInfo } from '../types';

  const actions = [
    { kind: 'region' as const, label: 'Region', hint: 'Select an area', icon: '▣' },
    { kind: 'fullscreen' as const, label: 'Fullscreen', hint: 'Entire screen', icon: '⛶' },
    { kind: 'window' as const, label: 'Window', hint: 'Focused window', icon: '❐' },
  ];

  const delays = [0, 1, 3, 5, 10];

  let elapsed = $state('00:00');
  let monitors = $state<MonitorInfo[]>([]);
  let delay = $state<number>(app.config?.captureDelaySeconds ?? 0);
  let monitor = $state<string>(app.config?.monitor ?? '');

  onMount(async () => {
    try {
      monitors = await api.listMonitors();
    } catch {
      monitors = [];
    }
  });

  $effect(() => {
    const started = app.recordingStartedAt;
    if (!started) {
      elapsed = '00:00';
      return;
    }
    const tick = () => {
      const total = Math.floor((Date.now() - started) / 1000);
      const mins = Math.floor(total / 60);
      const secs = total % 60;
      elapsed = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    };
    tick();
    const id = setInterval(tick, 500);
    return () => clearInterval(id);
  });

  const isGif = $derived(app.lastRecording?.format === 'gif');

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
    <h1>Record</h1>
    <p class="sub">Capture your screen to MP4, WebM or GIF. Press stop to finish.</p>
  </header>

  {#if app.recording}
    <div class="active">
      <div class="active-info">
        <span class="dot"></span>
        <div>
          <div class="active-title">Recording {app.recording.kind}</div>
          <div class="active-meta">
            {app.recording.format.toUpperCase()} · {app.recording.width || '?'}×{app.recording.height || '?'}
            · {app.recording.backend} · audio: {app.config?.record.audio ?? 'none'}
          </div>
        </div>
      </div>
      <div class="active-controls">
        <span class="elapsed">{elapsed}</span>
        <button class="stop" onclick={() => stopRecording()}>■ Stop</button>
      </div>
    </div>
  {:else}
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
        <button
          class="capture-card"
          onclick={() =>
            startRecording(action.kind, {
              delaySeconds: delay,
              monitor: monitor || undefined,
            })}
        >
          <span class="card-icon">{action.icon}</span>
          <span class="card-label">{action.label}</span>
          <span class="card-hint">{action.hint}</span>
        </button>
      {/each}
    </div>
    {#if app.config}
      <p class="hint">
        Output: <b>{app.config.record.format.toUpperCase()}</b>
        · {app.config.record.fps} fps{app.config.record.format === 'gif'
          ? ` · ${app.config.record.gifFps} fps GIF`
          : ''}
        · audio <b>{app.config.record.audio}</b>. Change this in Settings.
      </p>
    {/if}
  {/if}

  {#if app.lastRecording}
    <div class="last">
      <div class="last-head">
        <h2>Last recording</h2>
        <span class="pill">{app.lastRecording.format.toUpperCase()}</span>
      </div>
      <div class="last-body">
        {#if isGif}
          <img src={fileSrc(app.lastRecording.path)} alt={app.lastRecording.fileName} />
        {:else}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video src={fileSrc(app.lastRecording.path)} controls></video>
        {/if}
        <div class="last-meta">
          <div class="file">{app.lastRecording.fileName}</div>
          <dl>
            <div><dt>Size</dt><dd>{app.lastRecording.width} × {app.lastRecording.height}</dd></div>
            <div><dt>Bytes</dt><dd>{formatBytes(app.lastRecording.bytes)}</dd></div>
            <div><dt>Duration</dt><dd>{(app.lastRecording.durationMs / 1000).toFixed(1)}s</dd></div>
            <div><dt>Backend</dt><dd>{app.lastRecording.backend}</dd></div>
            <div><dt>When</dt><dd>{formatDate(app.lastRecording.createdAt)}</dd></div>
          </dl>
          <div class="last-actions">
            <button onclick={() => openPath(app.lastRecording!.path)}>Open</button>
            <button onclick={() => revealPath(app.lastRecording!.path)}>Folder</button>
            <button
              class="primary"
              onclick={() => uploadFile(app.lastRecording!.path)}
              disabled={app.uploading || app.config?.upload.uploader === 'none'}
            >
              {app.uploading ? 'Uploading…' : 'Upload'}
            </button>
          </div>
          {#if app.lastUpload}
            <div class="upload-link">
              <span class="dim">Uploaded via {app.lastUpload.uploader}</span>
              <code class="url">{app.lastUpload.url}</code>
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
  .capture-card:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
    background: var(--surface-3);
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
  }
  .hint {
    margin: 0 0 1.5rem;
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .active {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    background: var(--surface-2);
    border: 1px solid var(--danger);
    border-radius: 14px;
    padding: 1rem 1.2rem;
    margin-bottom: 1.5rem;
  }
  .active-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.25;
    }
  }
  .active-title {
    font-weight: 600;
  }
  .active-meta {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .active-controls {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }
  .elapsed {
    font-variant-numeric: tabular-nums;
    font-size: 1.15rem;
    font-weight: 600;
  }
  .stop {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
    font-weight: 600;
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
  .last-body img,
  .last-body video {
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
