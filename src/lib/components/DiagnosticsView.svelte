<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { app, toast } from '../state.svelte';
  import type { Diagnostics } from '../types';

  let diag = $state<Diagnostics | null>(null);

  onMount(async () => {
    try {
      diag = await api.diagnostics();
      app.diagnostics = diag;
    } catch (err) {
      toast(String(err), 'error');
    }
  });

  let report = $derived(diag ? JSON.stringify(diag, null, 2) : '');

  async function copyReport() {
    try {
      await api.copyText(report);
      toast('Diagnostics copied', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

<section class="view">
  <header class="view-head row">
    <div>
      <h1>Diagnostics</h1>
      <p class="sub">Environment and capture backend status.</p>
    </div>
    <button class="ghost" onclick={copyReport} disabled={!diag}>Copy report</button>
  </header>

  {#if !diag}
    <div class="empty">Running diagnostics…</div>
  {:else}
    <div class="cards">
      <div class="card">
        <h2>Environment</h2>
        <dl>
          <div><dt>Version</dt><dd>{diag.version}</dd></div>
          <div><dt>Session</dt><dd>{diag.sessionType}</dd></div>
          <div><dt>Desktop</dt><dd>{diag.desktop || 'unknown'}</dd></div>
          <div><dt>Wayland</dt><dd>{diag.wayland ? 'yes' : 'no'}</dd></div>
          <div><dt>X11</dt><dd>{diag.x11 ? 'yes' : 'no'}</dd></div>
          <div><dt>Active backend</dt><dd>{diag.activeBackend ?? 'none'}</dd></div>
          <div><dt>Hotkey backend</dt><dd>{diag.hotkeyBackend}</dd></div>
          <div><dt>Portal shortcuts</dt><dd>{diag.portalShortcuts ? 'available' : 'unavailable'}</dd></div>
        </dl>
      </div>

      <div class="card">
        <h2>Backends</h2>
        <ul class="backends">
          {#each diag.backends as backend (backend.id)}
            <li>
              <span class="dot" class:on={backend.available}></span>
              <div>
                <div class="b-name">{backend.id}</div>
                <div class="b-desc">{backend.name}</div>
              </div>
              <span class="b-state">{backend.available ? 'available' : 'unavailable'}</span>
            </li>
          {/each}
        </ul>
      </div>

      <div class="card wide">
        <h2>Paths</h2>
        <dl>
          <div><dt>Save directory</dt><dd class="path">{diag.saveDirectory}</dd></div>
          <div><dt>Config</dt><dd class="path">{diag.configPath}</dd></div>
          <div><dt>History</dt><dd class="path">{diag.historyPath}</dd></div>
        </dl>
      </div>
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
  .empty {
    padding: 3rem;
    text-align: center;
    color: var(--text-dim);
    background: var(--surface-2);
    border: 1px dashed var(--border);
    border-radius: 14px;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }
  .card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 1.1rem 1.2rem;
  }
  .card.wide {
    grid-column: 1 / -1;
  }
  .card h2 {
    margin: 0 0 0.8rem;
    font-size: 0.95rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-dim);
  }
  dl {
    margin: 0;
    display: grid;
    gap: 0.45rem;
  }
  dl > div {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: 0.86rem;
  }
  dt {
    color: var(--text-dim);
  }
  dd {
    margin: 0;
    text-align: right;
  }
  .path {
    word-break: break-all;
    font-size: 0.8rem;
  }
  .backends {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .backends li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--text-dim);
    flex-shrink: 0;
  }
  .dot.on {
    background: var(--success);
    box-shadow: 0 0 8px color-mix(in srgb, var(--success) 60%, transparent);
  }
  .b-name {
    font-size: 0.86rem;
    font-weight: 600;
  }
  .b-desc {
    font-size: 0.74rem;
    color: var(--text-dim);
  }
  .b-state {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
</style>
