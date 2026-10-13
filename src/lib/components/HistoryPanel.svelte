<script lang="ts">
  import { onMount } from 'svelte';
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import { app, toast, uploadFile } from '../state.svelte';
  import type { HistoryEntry } from '../types';

  let { onClose }: { onClose: () => void } = $props();

  type EntryWithDuration = HistoryEntry & { durationMs?: number };
  const durationOf = (entry: HistoryEntry): number | null => {
    const ms = (entry as EntryWithDuration).durationMs;
    return typeof ms === 'number' ? ms : null;
  };

  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let filter = $state<'all' | 'images' | 'recordings'>('all');
  let selectedEntry = $state<HistoryEntry | null>(null);

  onMount(async () => {
    await load();
  });

  async function load() {
    loading = true;
    try {
      entries = await api.listHistory();
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      loading = false;
    }
  }

  function filteredEntries() {
    switch (filter) {
      case 'images':
        return entries.filter(e => e.kind !== 'recording');
      case 'recordings':
        return entries.filter(e => e.kind === 'recording');
      default:
        return entries;
    }
  }

  async function copyEntry(entry: HistoryEntry) {
    try {
      await api.copyFileToClipboard(entry.path);
      toast('Copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function editEntry(entry: HistoryEntry) {
    if (entry.kind === 'recording') return;
    try {
      await api.openEditor(entry.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function deleteEntry(entry: HistoryEntry) {
    try {
      await api.deleteHistory(entry.id);
      entries = entries.filter(e => e.id !== entry.id);
      toast('Deleted', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function uploadEntry(entry: HistoryEntry) {
    try {
      await uploadFile(entry.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function openInFolder(entry: HistoryEntry) {
    api.revealPath(entry.path).catch((err: unknown) => toast(String(err), 'error'));
  }

  function pinEntry(entry: HistoryEntry) {
    if (entry.kind === 'recording') return;
    api.pinCapture(entry.path).catch((err: unknown) => toast(String(err), 'error'));
  }
</script>

<div class="panel-overlay" onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose()} role="button" tabindex="0" aria-label="Close history">
  <div class="panel history-panel" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-label="History" tabindex="-1">
    <header class="panel-head">
      <h2>History</h2>
      <div class="filters">
        <button class="filter-btn" class:active={filter === 'all'} onclick={() => filter = 'all'}>All</button>
        <button class="filter-btn" class:active={filter === 'images'} onclick={() => filter = 'images'}>Images</button>
        <button class="filter-btn" class:active={filter === 'recordings'} onclick={() => filter = 'recordings'}>Recordings</button>
      </div>
      <button class="close-btn" onclick={onClose} title="Close (Esc)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    </header>

    <div class="panel-body">
      {#if loading}
        <div class="loading">Loading history…</div>
      {:else if filteredEntries().length === 0}
        <div class="empty">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin-bottom: 0.5rem; opacity: 0.4;"><path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/><path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
          <p>No captures yet</p>
        </div>
      {:else}
        <div class="grid">
          {#each filteredEntries() as entry (entry.id)}
            <article class="card" class:selected={selectedEntry?.id === entry.id} onclick={() => selectedEntry = selectedEntry?.id === entry.id ? null : entry}>
              <div class="thumb-wrap">
                {#if entry.kind === 'recording'}
                  {@const recMs = durationOf(entry)}
                  <div class="thumb-video">
                    <span class="play-badge">▶</span>
                    {#if recMs !== null}<span class="duration">{(recMs / 1000).toFixed(1)}s</span>{/if}
                  </div>
                {:else}
                  <img src={fileSrc(entry.path)} alt={entry.fileName} loading="lazy" />
                {/if}
                <div class="card-overlay">
                  <button class="icon-btn" onclick={(e) => { e.stopPropagation(); editEntry(entry); }} title="Edit" class:hidden={entry.kind === 'recording'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                  </button>
                  <button class="icon-btn" onclick={(e) => { e.stopPropagation(); copyEntry(entry); }} title="Copy">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                  </button>
                  <button class="icon-btn" onclick={(e) => { e.stopPropagation(); uploadEntry(entry); }} title="Upload" class:disabled={app.uploading || app.config?.upload.uploader === 'none'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                  </button>
                  <button class="icon-btn" onclick={(e) => { e.stopPropagation(); pinEntry(entry); }} title="Pin" class:hidden={entry.kind === 'recording'}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 17v5"/><path d="M9 10a7 7 0 0114 0"/><path d="M3 21h18"/></svg>
                  </button>
                  <button class="icon-btn danger" onclick={(e) => { e.stopPropagation(); deleteEntry(entry); }} title="Delete">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                  </button>
                </div>
              </div>
              <div class="card-info">
                <p class="name">{entry.fileName}</p>
                <p class="meta">
                  {entry.width}×{entry.height} · {formatBytes(entry.bytes)} · {formatDate(entry.createdAt)}{#if durationOf(entry) !== null} · {((durationOf(entry) as number) / 1000).toFixed(1)}s{/if}
                </p>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>

    <footer class="panel-foot">
      <span class="count">{filteredEntries().length} item(s)</span>
      <button class="ghost" onclick={onClose}>Done</button>
    </footer>
  </div>
</div>

<style>
  .history-panel {
    right: 0;
    top: 0;
    bottom: 0;
    width: 420px;
    max-width: 100vw;
    display: flex;
    flex-direction: column;
    animation: slideIn 0.2s ease;
  }

  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2 {
    margin: 0;
    font-size: 0.95rem;
  }

  .filters {
    display: flex;
    gap: 4px;
  }

  .filter-btn {
    padding: 4px 10px;
    font-size: 0.7rem;
    border-radius: 6px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }

  .filter-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .filter-btn.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }

  .close-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }

  .close-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem;
  }

  .loading, .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--text-dim);
    font-size: 0.85rem;
    text-align: center;
  }

  .empty p {
    margin: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
  }

  .card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    cursor: pointer;
    transition: border-color 0.1s ease, box-shadow 0.1s ease;
  }

  .card:hover {
    border-color: var(--accent);
  }

  .card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .thumb-wrap {
    position: relative;
    aspect-ratio: 4 / 3;
    background: var(--surface-3);
    overflow: hidden;
  }

  .thumb-wrap img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.2s ease;
  }

  .card:hover .thumb-wrap img {
    transform: scale(1.05);
  }

  .thumb-video {
    width: 100%;
    height: 100%;
    background-size: cover;
    background-position: center;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 0.4rem;
  }

  .play-badge {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    border-radius: 50%;
    font-size: 0.8rem;
  }

  .duration {
    font-size: 0.65rem;
    font-variant-numeric: tabular-nums;
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .card-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .card:hover .card-overlay,
  .card.selected .card-overlay {
    opacity: 1;
  }

  .icon-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.9);
    color: #1a1a2e;
    cursor: pointer;
    transform: translateY(4px);
    transition: all 0.15s ease;
  }

  .card:hover .icon-btn,
  .card.selected .icon-btn {
    transform: translateY(0);
  }

  .icon-btn:hover {
    background: #fff;
    transform: translateY(-2px) scale(1.1);
  }

  .icon-btn.danger:hover {
    background: #ff4444;
    color: #fff;
  }

  .icon-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon-btn.hidden {
    display: none;
  }

  .card-info {
    padding: 0.5rem 0.6rem;
  }

  .name {
    margin: 0 0 0.2rem;
    font-size: 0.75rem;
    font-weight: 500;
    word-break: break-all;
  }

  .meta {
    margin: 0;
    font-size: 0.65rem;
    color: var(--text-dim);
  }

  .panel-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1rem;
    border-top: 1px solid var(--border);
  }

  .count {
    font-size: 0.72rem;
    color: var(--text-dim);
  }

  .ghost {
    padding: 4px 12px;
    font-size: 0.78rem;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .ghost:hover {
    background: var(--surface-2);
  }
</style>