<script lang="ts">
  import { api, fileSrc, formatBytes } from '../api';
  import { app, reloadHistory, toast, uploadFile } from '../state.svelte';
  import type { CaptureResult } from '../types';

  let { result, onClose }: { result: CaptureResult; onClose: () => void } = $props();

  const hasFile = $derived(!!result.path);

  async function run(label: string, fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (err) {
      toast(`${label} failed: ${err}`, 'error');
    }
  }

  function deleteCapture() {
    api
      .deleteHistory(result.id)
      .then(() => {
        reloadHistory();
        toast('Deleted', 'success');
        onClose();
      })
      .catch((err: unknown) => toast(String(err), 'error'));
  }

  async function saveAs() {
    try {
      const dest = await api.saveCaptureAs(result.path);
      toast(`Saved to ${dest}`, 'success');
      onClose();
    } catch (err) {
      const message = String(err);
      if (!message.toLowerCase().includes('cancel')) toast(String(err), 'error');
    }
  }
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close quick task menu"
>
  <div
    class="menu"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Quick task menu"
    tabindex="-1"
  >
    <div class="menu-head">
      <h2>Quick task menu</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>
    {#if hasFile}
      <img class="thumb" src={fileSrc(result.path)} alt={result.fileName} />
      <div class="fname">{result.fileName} · {formatBytes(result.bytes)}</div>
    {:else}
      <p class="hint">Enable “Save image to file” to use file actions for this capture.</p>
    {/if}
    <div class="actions">
      <button disabled={!hasFile} onclick={() => run('Open', () => api.openPath(result.path)).then(onClose)}>Open image</button>
      <button disabled={!hasFile} onclick={() => run('Reveal', () => api.revealPath(result.path)).then(onClose)}>Open folder</button>
      <button disabled={!hasFile} onclick={() => run('Copy', () => api.copyFileToClipboard(result.path).then(() => toast('Copied to clipboard', 'success')))}>Copy image</button>
      <button disabled={!hasFile} onclick={() => run('Edit', () => api.openEditor(result.path)).then(onClose)}>Annotate image</button>
      <button disabled={!hasFile || app.uploading} onclick={() => uploadFile(result.path).then(onClose)}>Upload image</button>
      <button disabled={!hasFile} onclick={() => run('Pin', () => api.pinCapture(result.path)).then(onClose)}>Pin to screen</button>
      <button disabled={!hasFile} onclick={saveAs}>Save as…</button>
      <button class="danger" onclick={deleteCapture}>Delete</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1500;
  }
  .menu {
    width: min(340px, 92vw);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0.9rem;
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
  }
  .menu-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
  }
  .menu-head h2 {
    margin: 0;
    font-size: 0.9rem;
  }
  .mini {
    padding: 0.2rem 0.6rem;
    font-size: 0.72rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }
  .mini:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .thumb {
    width: 100%;
    max-height: 180px;
    object-fit: contain;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: #0d0f14;
  }
  .fname {
    margin: 0.4rem 0 0.2rem;
    font-size: 0.72rem;
    color: var(--text-dim);
    word-break: break-all;
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-top: 0.5rem;
  }
  .actions button {
    padding: 0.42rem 0.7rem;
    font-size: 0.8rem;
    text-align: left;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .actions button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .actions button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .actions button.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
