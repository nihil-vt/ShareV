<script lang="ts">
  import { app, startAutoCapture, stopAutoCapture, toast } from '../state.svelte';
  import { api } from '../api';
  import type { CaptureKind, MonitorInfo } from '../types';
  import { onMount } from 'svelte';

  let { onClose }: { onClose: () => void } = $props();

  let kind = $state<CaptureKind>('region');
  let interval = $state(3);
  let maxCount = $state(0);
  let maxMinutes = $state(0);
  let changeOnly = $state(false);
  let monitor = $state('');
  let monitors = $state<MonitorInfo[]>([]);
  let busy = $state(false);

  onMount(async () => {
    try {
      monitors = await api.listMonitors();
    } catch {
      monitors = [];
    }
    const auto = app.config?.auto;
    if (auto) {
      interval = Math.max(1, Math.round(auto.intervalMs / 1000));
      maxCount = auto.maxCount;
      maxMinutes = Math.round(auto.maxDurationSeconds / 60);
      changeOnly = auto.changeDetection;
    }
    monitor = app.config?.monitor ?? '';
  });

  async function start() {
    if (busy) return;
    busy = true;
    try {
      await startAutoCapture(kind, {
        intervalMs: Math.max(500, Math.round(interval * 1000)),
        changeDetection: changeOnly,
        maxCount: Math.max(0, Math.round(maxCount)),
        maxDurationSeconds: Math.max(0, Math.round(maxMinutes * 60)),
        monitor: kind === 'fullscreen' && monitor ? monitor : undefined,
      });
      onClose();
    } catch (err) {
      toast(String(err), 'error');
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
  aria-label="Close auto capture"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Auto capture"
    tabindex="-1"
  >
    <div class="head">
      <h2>Auto capture</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    {#if app.autoCapture}
      <p class="running">
        Running: {app.autoCapture.kind} · every {(app.autoCapture.intervalMs / 1000).toFixed(1)}s ·
        {app.autoCapture.count} saved
      </p>
      <button class="stop" onclick={() => stopAutoCapture().then(onClose)}>Stop auto capture</button>
    {:else}
      <label class="field">
        <span>Target</span>
        <select bind:value={kind}>
          <option value="region">Region</option>
          <option value="fullscreen">Fullscreen</option>
          <option value="window">Window</option>
        </select>
      </label>
      {#if kind === 'fullscreen'}
        <label class="field">
          <span>Monitor</span>
          <select bind:value={monitor}>
            <option value="">Auto (focused)</option>
            {#each monitors as m (m.name)}
              <option value={m.name}>{m.focused ? '● ' : ''}{m.name}</option>
            {/each}
          </select>
        </label>
      {/if}
      <label class="field">
        <span>Interval (seconds)</span>
        <input type="number" min="0.5" step="0.5" bind:value={interval} />
      </label>
      <div class="row">
        <label class="field">
          <span>Max captures (0 = unlimited)</span>
          <input type="number" min="0" step="1" bind:value={maxCount} />
        </label>
        <label class="field">
          <span>Time limit, minutes (0 = none)</span>
          <input type="number" min="0" step="1" bind:value={maxMinutes} />
        </label>
      </div>
      <label class="check">
        <input type="checkbox" bind:checked={changeOnly} />
        <span>Only save when the image changes</span>
      </label>
      <button class="primary" disabled={busy} onclick={start}>
        {busy ? 'Starting…' : 'Start auto capture'}
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
    width: min(420px, 92vw);
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
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-dim);
  }
  .field select,
  .field input {
    padding: 0.4rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.82rem;
  }
  .row {
    display: flex;
    gap: 0.6rem;
  }
  .row .field {
    flex: 1;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--accent);
  }
  .primary {
    padding: 0.5rem;
    border-radius: 8px;
    border: 1px solid var(--accent);
    background: var(--accent);
    color: #fff;
    font-size: 0.84rem;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.55;
    cursor: progress;
  }
  .stop {
    padding: 0.5rem;
    border-radius: 8px;
    border: 1px solid var(--danger);
    background: transparent;
    color: var(--danger);
    font-size: 0.84rem;
    font-weight: 600;
    cursor: pointer;
  }
  .stop:hover {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
  .running {
    font-size: 0.82rem;
    color: var(--text-dim);
  }
</style>
