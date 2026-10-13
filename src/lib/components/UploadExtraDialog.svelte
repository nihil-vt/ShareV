<script lang="ts">
  import { app, uploadText, uploadUrl } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let tab = $state<'text' | 'url'>('text');
  let text = $state('');
  let url = $state('');
  let busy = $state(false);

  async function submit() {
    if (busy || app.uploading) return;
    busy = true;
    try {
      if (tab === 'text') {
        if (!text.trim()) return;
        await uploadText(text);
      } else {
        if (!url.trim()) return;
        await uploadUrl(url.trim());
      }
      onClose();
    } finally {
      busy = false;
    }
  }
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close upload dialog"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Upload text or URL"
    tabindex="-1"
  >
    <div class="head">
      <h2>Upload</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === 'text'} class:active={tab === 'text'} onclick={() => (tab = 'text')}>Text…</button>
      <button role="tab" aria-selected={tab === 'url'} class:active={tab === 'url'} onclick={() => (tab = 'url')}>From URL…</button>
    </div>

    {#if tab === 'text'}
      <p class="hint">Posts the text through the configured uploader. Image-only destinations (e.g. Imgur) reject plain text.</p>
      <textarea bind:value={text} rows="8" placeholder="Paste text to upload…"></textarea>
      <button class="primary" disabled={busy || !text.trim()} onclick={submit}>
        {busy ? 'Uploading…' : 'Upload text'}
      </button>
    {:else}
      <p class="hint">Downloads the file, then uploads it through the configured uploader.</p>
      <input type="text" bind:value={url} placeholder="https://example.com/file.png" aria-label="File URL" />
      <button class="primary" disabled={busy || !url.trim()} onclick={submit}>
        {busy ? 'Uploading…' : 'Download & upload'}
      </button>
    {/if}
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
  .dialog {
    width: min(460px, 92vw);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .head h2 {
    margin: 0;
    font-size: 0.92rem;
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
  .tabs {
    display: flex;
    gap: 0.3rem;
  }
  .tabs button {
    flex: 1;
    padding: 0.35rem;
    font-size: 0.78rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }
  .tabs button.active {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .hint {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  textarea,
  input[type='text'] {
    padding: 0.45rem 0.55rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.82rem;
    font-family: inherit;
  }
  textarea {
    resize: vertical;
  }
  button.primary {
    padding: 0.45rem;
    border-radius: 8px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: #fff;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
