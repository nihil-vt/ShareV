<script lang="ts">
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import { app, reloadHistory, toast, uploadFile } from '../state.svelte';
  import type { HistoryEntry } from '../types';

  let query = $state('');
  let selected = $state<HistoryEntry | null>(null);

  function isVideo(entry: HistoryEntry): boolean {
    return /\.(mp4|webm)$/i.test(entry.path);
  }

  const filtered = $derived(
    app.history.filter((entry) => {
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return (
        entry.fileName.toLowerCase().includes(q) || entry.kind.toLowerCase().includes(q)
      );
    }),
  );

  async function copy(entry: HistoryEntry) {
    try {
      await api.copyFileToClipboard(entry.path);
      toast('Copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function open(entry: HistoryEntry) {
    try {
      await api.openPath(entry.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function reveal(entry: HistoryEntry) {
    try {
      await api.revealPath(entry.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function edit(entry: HistoryEntry) {
    app.editing = entry;
    app.view = 'editor';
  }

  async function upload(entry: HistoryEntry) {
    const outcome = await uploadFile(entry.path);
    // Re-point the selection at the refreshed entry so its URL shows up.
    if (outcome && selected?.id === entry.id) {
      const fresh = app.history.find((e) => e.id === entry.id);
      if (fresh) selected = fresh;
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

  async function openLink(url: string) {
    try {
      await api.openPath(url);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function remove(entry: HistoryEntry) {
    try {
      await api.deleteHistory(entry.id);
      if (selected?.id === entry.id) selected = null;
      await reloadHistory();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function clearAll() {
    if (app.history.length === 0) return;
    if (!confirm('Delete all history entries? The image files are kept.')) return;
    try {
      await api.clearHistory();
      selected = null;
      await reloadHistory();
      toast('History cleared', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

<section class="view">
  <header class="view-head row">
    <div>
      <h1>History</h1>
      <p class="sub">{app.history.length} item{app.history.length === 1 ? '' : 's'}</p>
    </div>
    <div class="tools">
      <input class="search" type="search" placeholder="Filter…" bind:value={query} />
      <button class="ghost" onclick={clearAll}>Clear</button>
    </div>
  </header>

  {#if app.history.length === 0}
    <div class="empty">No captures yet. Head to <b>Capture</b> to take one.</div>
  {:else}
    <div class="history-layout">
      <ul class="entries">
        {#each filtered as entry (entry.id)}
          <li>
            <button
              class="entry"
              class:active={selected?.id === entry.id}
              onclick={() => (selected = entry)}
            >
              {#if isVideo(entry)}
                <video class="thumb" src={fileSrc(entry.path)} muted preload="metadata"></video>
              {:else}
                <img loading="lazy" src={fileSrc(entry.path)} alt="" />
              {/if}
              <div class="meta">
                <span class="name">{entry.fileName}</span>
                <span class="dim">{entry.width}×{entry.height} · {formatBytes(entry.bytes)}</span>
                <span class="dim">{formatDate(entry.createdAt)}</span>
              </div>
            </button>
          </li>
        {/each}
        {#if filtered.length === 0}
          <li class="empty-filter">No matches for “{query}”.</li>
        {/if}
      </ul>

      {#if selected}
        <div class="preview">
          {#if isVideo(selected)}
            <!-- svelte-ignore a11y_media_has_caption -->
            <video src={fileSrc(selected.path)} controls></video>
          {:else}
            <img src={fileSrc(selected.path)} alt={selected.fileName} />
          {/if}
          <div class="preview-meta">
            <div class="name">{selected.fileName}</div>
            <div class="dim">
              {selected.kind} · {selected.width}×{selected.height} · {formatBytes(selected.bytes)} ·
              {selected.backend}
            </div>
            <div class="preview-actions">
              {#if !isVideo(selected)}
                <button class="primary" onclick={() => edit(selected!)}>Edit</button>
              {/if}
              <button onclick={() => open(selected!)}>Open</button>
              <button onclick={() => reveal(selected!)}>Folder</button>
              <button onclick={() => copy(selected!)}>Copy</button>
              <button
                onclick={() => upload(selected!)}
                disabled={app.uploading || app.config?.upload.uploader === 'none'}
              >
                {app.uploading ? 'Uploading…' : 'Upload'}
              </button>
              <button class="danger" onclick={() => remove(selected!)}>Remove</button>
            </div>
            {#if selected.url}
              <div class="preview-url">
                <code>{selected.url}</code>
                <button class="ghost" onclick={() => openLink(selected!.url!)}>Open</button>
                <button class="ghost" onclick={() => copyLink(selected!.url!)}>Copy link</button>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
  }
  .tools {
    display: flex;
    gap: 0.5rem;
  }
  .search {
    width: 200px;
  }
  .empty {
    padding: 3rem;
    text-align: center;
    color: var(--text-dim);
    background: var(--surface-2);
    border: 1px dashed var(--border);
    border-radius: 14px;
  }
  .history-layout {
    display: grid;
    grid-template-columns: minmax(280px, 380px) 1fr;
    gap: 1rem;
    align-items: start;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: calc(100vh - 220px);
    overflow-y: auto;
  }
  .entry {
    display: flex;
    gap: 0.7rem;
    width: 100%;
    text-align: left;
    padding: 0.5rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    cursor: pointer;
  }
  .entry:hover,
  .entry.active {
    border-color: var(--accent);
    background: var(--surface-3);
  }
  .entry img,
  .entry video.thumb {
    width: 72px;
    height: 48px;
    object-fit: cover;
    border-radius: 6px;
    background: var(--surface-3);
    flex-shrink: 0;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 0.86rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dim {
    font-size: 0.74rem;
    color: var(--text-dim);
  }
  .empty-filter {
    color: var(--text-dim);
    font-size: 0.85rem;
    padding: 0.5rem;
  }
  .preview {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 0.85rem;
    position: sticky;
    top: 0;
  }
  .preview img,
  .preview video {
    width: 100%;
    max-height: 62vh;
    object-fit: contain;
    border-radius: 10px;
    background: var(--surface-3);
  }
  .preview-meta {
    margin-top: 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .preview-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.4rem;
  }
  .preview-url {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-top: 0.6rem;
    padding-top: 0.6rem;
    border-top: 1px solid var(--border);
  }
  .preview-url code {
    background: var(--surface-3);
    padding: 0.15rem 0.45rem;
    border-radius: 6px;
    font-size: 0.76rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .danger {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
  }
</style>
