<script lang="ts">
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let tab = $state<'encode' | 'decode'>('encode');
  let text = $state('');
  let size = $state(320);
  let working = $state(false);
  let resultPath = $state<string | null>(null);
  let decodePath = $state('');
  let decoded = $state<string[]>([]);

  async function encode() {
    if (!text.trim() || working) return;
    working = true;
    try {
      resultPath = await api.qrEncode(text, size);
      toast('QR code saved', 'success');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function browse() {
    try {
      const picked = await api.pickFile();
      if (picked) {
        decodePath = picked;
        decoded = [];
      }
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function decode() {
    if (!decodePath || working) return;
    working = true;
    try {
      decoded = await api.qrDecode(decodePath);
      if (decoded.length === 0) toast('No QR code found', 'error');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function copy(value: string) {
    try {
      await api.copyText(value);
      toast('Copied', 'success');
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
  aria-label="Close QR tool"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="QR code tool"
    tabindex="-1"
  >
    <div class="head">
      <h2>QR code</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="tabs" role="tablist">
      <button
        role="tab"
        aria-selected={tab === 'encode'}
        class:on={tab === 'encode'}
        onclick={() => (tab = 'encode')}
      >
        Create
      </button>
      <button
        role="tab"
        aria-selected={tab === 'decode'}
        class:on={tab === 'decode'}
        onclick={() => (tab = 'decode')}
      >
        Read
      </button>
    </div>

    {#if tab === 'encode'}
      <label class="field">
        <span>Text / URL</span>
        <textarea bind:value={text} rows="3" placeholder="https://example.com"></textarea>
      </label>
      <label class="field">
        <span>Size ({size}px)</span>
        <input type="range" min="64" max="2048" step="32" bind:value={size} />
      </label>
      <button class="primary" disabled={working || !text.trim()} onclick={encode}>
        {working ? 'Encoding…' : 'Generate and save'}
      </button>
      {#if resultPath}
        <div class="done">
          <code>{resultPath}</code>
          <button onclick={() => api.openPath(resultPath!)}>Open</button>
        </div>
      {/if}
    {:else}
      <div class="file-row">
        <input type="text" value={decodePath.split('/').pop()} readonly placeholder="No image chosen" aria-label="Chosen image" />
        <button onclick={browse}>Browse…</button>
      </div>
      <button class="primary" disabled={working || !decodePath} onclick={decode}>
        {working ? 'Reading…' : 'Read QR codes'}
      </button>
      {#if decoded.length > 0}
        <ul class="decoded">
          {#each decoded as d (d)}
            <li>
              <code>{d}</code>
              <button onclick={() => copy(d)}>Copy</button>
            </li>
          {/each}
        </ul>
      {/if}
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
    padding: 1rem;
  }
  .dialog {
    width: min(460px, 100%);
    max-height: 88vh;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0.9rem;
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
    background: var(--surface-2);
    color: var(--text-dim);
    cursor: pointer;
  }
  .tabs button.on {
    background: var(--surface-3);
    color: var(--text);
    border-color: var(--accent);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  textarea,
  input[type='text'] {
    padding: 0.4rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
    font-family: inherit;
    resize: vertical;
  }
  input[type='range'] {
    width: 100%;
    accent-color: var(--accent);
  }
  input[type='range'],
  .file-row input {
    padding: 0.4rem 0.5rem;
  }
  .file-row {
    display: flex;
    gap: 0.4rem;
  }
  .file-row input {
    flex: 1;
    min-width: 0;
  }
  button {
    padding: 0.4rem 0.7rem;
    font-size: 0.78rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
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
  .done {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .done code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--surface-3);
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.72rem;
  }
  .decoded {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .decoded li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .decoded code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--surface-3);
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.72rem;
  }
</style>
