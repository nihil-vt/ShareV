<script lang="ts">
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import { app, doCapture, toast, uploadFile } from '../state.svelte';

  const actions = [
    { kind: 'region' as const, label: 'Region', hint: 'Select an area', icon: '▣' },
    { kind: 'fullscreen' as const, label: 'Fullscreen', hint: 'Entire screen', icon: '⛶' },
    { kind: 'window' as const, label: 'Window', hint: 'Focused window', icon: '❐' },
  ];

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

  <div class="capture-grid">
    {#each actions as action (action.kind)}
      <button class="capture-card" disabled={app.busy} onclick={() => doCapture(action.kind)}>
        <span class="card-icon">{action.icon}</span>
        <span class="card-label">{action.label}</span>
        <span class="card-hint">{action.hint}</span>
        {#if app.busy && app.busyKind === action.kind}
          <span class="card-spinner">working…</span>
        {/if}
      </button>
    {/each}
  </div>

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
  }
  .card-spinner {
    font-size: 0.75rem;
    color: var(--accent);
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
