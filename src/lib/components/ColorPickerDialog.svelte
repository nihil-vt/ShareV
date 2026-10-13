<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { toast } from '../state.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let imgEl = $state<HTMLImageElement | null>(null);
  let loading = $state(true);
  let picked = $state<{ x: number; y: number; r: number; g: number; b: number } | null>(null);

  const hex = $derived(
    picked ? `#${[picked.r, picked.g, picked.b].map((v) => v.toString(16).padStart(2, '0')).join('')}` : null,
  );
  const rgb = $derived(picked ? `rgb(${picked.r}, ${picked.g}, ${picked.b})` : null);

  onMount(async () => {
    try {
      const shot = await api.grabPixels();
      if (!imgEl) return;
      await new Promise<void>((resolve, reject) => {
        imgEl!.onload = () => resolve();
        imgEl!.onerror = () => reject(new Error('could not decode the frame'));
        imgEl!.src = shot.dataUrl;
      });
    } catch (err) {
      toast(String(err), 'error');
      onClose();
    } finally {
      loading = false;
    }
  });

  function pick(e: MouseEvent) {
    if (!imgEl) return;
    const rect = imgEl.getBoundingClientRect();
    // Keyboard activation reports 0,0 — fall back to the center pixel.
    const clientX = e.clientX === 0 && e.clientY === 0 ? rect.left + rect.width / 2 : e.clientX;
    const clientY = e.clientX === 0 && e.clientY === 0 ? rect.top + rect.height / 2 : e.clientY;
    const x = Math.floor((clientX - rect.left) * (imgEl.naturalWidth / rect.width));
    const y = Math.floor((clientY - rect.top) * (imgEl.naturalHeight / rect.height));
    const canvas = document.createElement('canvas');
    canvas.width = imgEl.naturalWidth;
    canvas.height = imgEl.naturalHeight;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;
    ctx.drawImage(imgEl, 0, 0);
    const [r, g, b] = ctx.getImageData(x, y, 1, 1).data;
    picked = { x, y, r, g, b };
  }

  async function copy(value: string | null, label: string) {
    if (!value) return;
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
  aria-label="Close color picker"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Screen color picker"
    tabindex="-1"
  >
    <div class="head">
      <h2>Screen color picker</h2>
      <button class="mini" onclick={onClose}>Close</button>
    </div>

    {#if loading}
      <p class="hint">Grabbing the screen…</p>
    {:else}
      <button
        class="shot-btn"
        onclick={pick}
        aria-label="Fullscreen capture — activate to pick the center pixel, or click to pick any pixel"
      >
        <img
          bind:this={imgEl}
          class="shot"
          alt="Fullscreen capture — click to pick a color"
        />
      </button>
      {#if picked}
        <div class="result">
          <span class="swatch" style="background:{hex}"></span>
          <code>{hex}</code>
          <button onclick={() => copy(hex, 'HEX')}>Copy HEX</button>
          <code>{rgb}</code>
          <button onclick={() => copy(rgb, 'RGB')}>Copy RGB</button>
          <span class="coords">{picked.x}, {picked.y}</span>
        </div>
      {:else}
        <p class="hint">Click anywhere on the image to pick that pixel's color.</p>
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
    width: min(720px, 100%);
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
  .shot-btn {
    display: block;
    width: 100%;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: none;
    cursor: crosshair;
  }
  .shot {
    display: block;
    width: 100%;
    cursor: crosshair;
  }
  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .result {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .swatch {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  code {
    background: var(--surface-3);
    padding: 0.25rem 0.5rem;
    border-radius: 6px;
    font-size: 0.78rem;
  }
  .coords {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .result button {
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .result button:hover {
    border-color: var(--accent);
  }
</style>
