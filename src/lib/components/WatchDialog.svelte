<script lang="ts">
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let folders = $state<string[]>([]);
  let working = $state(false);

  async function refresh() {
    try {
      folders = await api.watchList();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function add() {
    working = true;
    try {
      const picked = await api.pickFolder();
      if (picked) {
        const added = await api.watchAdd(picked);
        toast(added ? `Watching ${picked}` : 'Already watched', added ? 'success' : 'error');
        await refresh();
      }
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      working = false;
    }
  }

  async function remove(folder: string) {
    try {
      const removed = await api.watchRemove(folder);
      toast(removed ? 'Stopped watching' : 'Not watched', removed ? 'success' : 'error');
      await refresh();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  void refresh();
</script>

<div
  class="overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close watch folders"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Watch folders"
    tabindex="-1"
  >
    <div class="head">
      <h2>Watch folders</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <p class="hint">
      New images dropped into a watched folder land in the history, like a
      ShareX watch folder.
    </p>

    <button onclick={add} disabled={working}>
      {working ? 'Opening…' : 'Add folder…'}
    </button>

    {#if folders.length === 0}
      <p class="hint">No folders watched yet.</p>
    {:else}
      <ul class="folders">
        {#each folders as folder (folder)}
          <li>
            <span class="fname" title={folder}>{folder}</span>
            <button onclick={() => remove(folder)} aria-label="Stop watching {folder}">
              Stop
            </button>
          </li>
        {/each}
      </ul>
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
    font-size: 0.76rem;
    color: var(--text-dim);
    line-height: 1.4;
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
  .folders {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .folders li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .fname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.76rem;
    background: var(--surface-2);
    padding: 0.35rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
  }
</style>
