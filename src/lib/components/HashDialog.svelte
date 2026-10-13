<script lang="ts">
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let path = $state('');
  let fileName = $state('');
  let hashes = $state<Record<string, string> | null>(null);
  let working = $state(false);

  const rows = $derived(
    hashes
      ? ([
          ['MD5', hashes.md5],
          ['SHA-256', hashes.sha256],
          ['SHA-512', hashes.sha512],
          ['BLAKE3', hashes.blake3],
        ] as const)
      : [],
  );

  async function browse() {
    try {
      const picked = await api.pickFile();
      if (picked) {
        path = picked;
        fileName = picked.split('/').pop() ?? picked;
        hashes = null;
      }
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function compute() {
    if (!path) return;
    working = true;
    try {
      hashes = await api.hashFile(path);
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function copy(value: string, label: string) {
    try {
      await api.copyText(value);
      toast(`${label} copied`, 'success');
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
  aria-label="Close hash checker"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Hash checker"
    tabindex="-1"
  >
    <div class="head">
      <h2>Hash checker</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="file-row">
      <input type="text" value={fileName} readonly placeholder="No file chosen" aria-label="Chosen file" />
      <button onclick={browse}>Browse…</button>
    </div>

    <button class="primary" disabled={working || !path} onclick={compute}>
      {working ? 'Hashing…' : 'Compute hashes'}
    </button>

    {#if rows.length > 0}
      <dl class="hashes">
        {#each rows as [label, value] (label)}
          <div class="hash-row">
            <dt>{label}</dt>
            <dd><code>{value}</code></dd>
            <button onclick={() => copy(value, label)}>Copy</button>
          </div>
        {/each}
      </dl>
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
    width: min(560px, 100%);
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
  .file-row {
    display: flex;
    gap: 0.4rem;
  }
  .file-row input {
    flex: 1;
    min-width: 0;
  }
  input[type='text'] {
    padding: 0.4rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
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
  .hashes {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .hash-row {
    display: grid;
    grid-template-columns: 4.2rem 1fr auto;
    gap: 0.5rem;
    align-items: center;
  }
  .hash-row dt {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-dim);
  }
  .hash-row dd {
    margin: 0;
    min-width: 0;
  }
  .hash-row code {
    display: block;
    overflow-x: auto;
    white-space: nowrap;
    background: var(--surface-3);
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.7rem;
    scrollbar-width: thin;
  }
</style>
