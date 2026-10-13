<script lang="ts">
  import type { ImageMetadata } from '../types';
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let path = $state('');
  let working = $state(false);
  let meta = $state<ImageMetadata | null>(null);

  const tagGroups = $derived.by(() => {
    if (!meta) return [] as { label: string; rows: [string, string][] }[];
    const exif = Object.entries(meta.tags).filter(([k]) =>
      /^(exif|gps|interop|maker|unknown)/i.test(k) === false,
    );
    const rest = Object.entries(meta.tags).filter(([k]) =>
      /^(exif|gps|interop|maker|unknown)/i.test(k),
    );
    return [
      { label: 'Image', rows: [
        ['Format', meta.format],
        ['Dimensions', `${meta.width} × ${meta.height}`],
        ['Size', `${meta.bytes.toLocaleString()} bytes`],
        ['Modified', new Date(meta.modified * 1000).toLocaleString()],
      ] as [string, string][] },
      { label: 'Tags', rows: exif as [string, string][] },
      { label: 'Other', rows: rest as [string, string][] },
    ].filter((g) => g.rows.length > 0);
  });

  async function browse() {
    try {
      const picked = await api.pickFile();
      if (picked) {
        path = picked;
        meta = null;
      }
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function read() {
    if (!path || working) return;
    working = true;
    try {
      meta = await api.metadataImage(path);
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
  aria-label="Close metadata viewer"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Image metadata"
    tabindex="-1"
  >
    <div class="head">
      <h2>Image metadata</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    <div class="file-row">
      <input type="text" value={path.split('/').pop()} readonly placeholder="No file chosen" aria-label="Chosen file" />
      <button onclick={browse}>Browse…</button>
    </div>

    <button class="primary" disabled={working || !path} onclick={read}>
      {working ? 'Reading…' : 'Read metadata'}
    </button>

    {#if tagGroups.length > 0}
      {#each tagGroups as group (group.label)}
        <table>
          <caption>{group.label}</caption>
          <tbody>
            {#each group.rows as [k, v] (k)}
              <tr>
                <th scope="row">{k}</th>
                <td>
                  <code>{v}</code>
                  <button class="tiny" onclick={() => copy(v, k)}>Copy</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/each}
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
    width: min(520px, 100%);
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
  button.tiny {
    padding: 0.1rem 0.4rem;
    font-size: 0.66rem;
    background: transparent;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.74rem;
  }
  caption {
    text-align: left;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    padding: 0.3rem 0;
  }
  th,
  td {
    text-align: left;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }
  th {
    width: 34%;
    color: var(--text-dim);
    font-weight: 600;
  }
  td {
    display: table-cell;
  }
  td code {
    overflow-wrap: anywhere;
    background: var(--surface-3);
    padding: 0.15rem 0.35rem;
    border-radius: 5px;
    font-size: 0.72rem;
  }
  td .tiny {
    margin-left: 0.4rem;
  }
</style>
