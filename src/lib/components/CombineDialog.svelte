<script lang="ts">
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let files = $state<string[]>([]);
  let direction = $state<'horizontal' | 'vertical'>('horizontal');
  let spacing = $state(0);
  let background = $state('#000000');
  let working = $state(false);
  let result = $state<string | null>(null);

  async function add() {
    try {
      const picked = await api.pickFile();
      if (picked && !files.includes(picked)) files = [...files, picked];
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function combine() {
    if (files.length < 2 || working) return;
    working = true;
    try {
      result = await api.combineImages(files, direction, Math.max(0, Math.round(spacing)), background);
      toast('Combined image saved', 'success');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function openResult() {
    if (!result) return;
    try {
      await api.openPath(result);
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
  aria-label="Close image combiner"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Combine images"
    tabindex="-1"
  >
    <div class="head">
      <h2>Combine images</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="file-list">
      {#if files.length === 0}
        <p class="hint">Add at least two images to join.</p>
      {:else}
        {#each files as f, i (f)}
          <div class="file-row">
            <span class="fname">{f.split('/').pop()}</span>
            <button onclick={() => (files = files.filter((_, j) => j !== i))} aria-label="Remove {f}">×</button>
          </div>
        {/each}
      {/if}
    </div>
    <button onclick={add}>Add image…</button>

    <div class="opts">
      <label class="field">
        <span>Direction</span>
        <select bind:value={direction}>
          <option value="horizontal">Side by side</option>
          <option value="vertical">Stacked</option>
        </select>
      </label>
      <label class="field">
        <span>Spacing (px)</span>
        <input type="number" min="0" max="200" bind:value={spacing} />
      </label>
      <label class="field">
        <span>Background</span>
        <input type="color" bind:value={background} />
      </label>
    </div>

    <button class="primary" disabled={working || files.length < 2} onclick={combine}>
      {working ? 'Combining…' : 'Combine'}
    </button>

    {#if result}
      <div class="done">
        <code>{result}</code>
        <button onclick={openResult}>Open</button>
      </div>
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
    width: min(480px, 100%);
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
  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .file-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 180px;
    overflow-y: auto;
  }
  .file-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.5rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    font-size: 0.76rem;
  }
  .fname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opts {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .field {
    flex: 1;
    min-width: 120px;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  select,
  input[type='number'] {
    padding: 0.35rem 0.45rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
  }
  input[type='color'] {
    width: 100%;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface-2);
    cursor: pointer;
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
</style>
