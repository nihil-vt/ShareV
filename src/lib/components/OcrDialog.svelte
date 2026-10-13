<script lang="ts">
  import { api } from '../api';
  import { app, toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let path = $state('');
  let language = $state(app.config?.ocrLanguage ?? 'eng');
  let result = $state<string | null>(null);
  let working = $state(false);

  async function browse() {
    try {
      const picked = await api.pickFile();
      if (picked) {
        path = picked;
        result = null;
      }
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function recognize() {
    if (!path) {
      toast('Choose an image first', 'info');
      return;
    }
    if (!app.config?.features.ocr) {
      toast('Enable the OCR feature in Settings first', 'info');
      return;
    }
    working = true;
    try {
      const text = await api.ocrRecognize(path, language.trim() || 'eng');
      result = text;
      if (!text.trim()) toast('No text recognized', 'info');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function copyResult() {
    if (!result) return;
    try {
      await api.copyText(result);
      toast('Copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close OCR"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="OCR image"
    tabindex="-1"
  >
    <div class="head">
      <h2>OCR image…</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="file-row">
      <input type="text" value={path} readonly placeholder="No file chosen" aria-label="Chosen image" />
      <button onclick={browse}>Browse…</button>
    </div>

    <label class="field">
      <span>Language code (Tesseract, e.g. eng)</span>
      <input type="text" bind:value={language} placeholder="eng" />
    </label>

    <button class="primary" disabled={working || !path} onclick={recognize}>
      {working ? 'Recognizing…' : 'Recognize text'}
    </button>

    {#if result !== null}
      <textarea readonly value={result} rows="8" aria-label="Recognized text"></textarea>
      <button disabled={!result.trim()} onclick={copyResult}>Copy result</button>
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
    max-height: 86vh;
    overflow-y: auto;
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
  .file-row {
    display: flex;
    gap: 0.4rem;
  }
  .file-row input {
    flex: 1;
    min-width: 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-dim);
  }
  input[type='text'],
  textarea {
    padding: 0.4rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.82rem;
  }
  textarea {
    resize: vertical;
    font-family: inherit;
  }
  button {
    padding: 0.42rem 0.7rem;
    font-size: 0.8rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
    font-weight: 600;
  }
</style>
