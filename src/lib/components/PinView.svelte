<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, fileSrc } from '../api';

  const win = getCurrentWindow();

  let info = $state<{ path: string; width: number; height: number } | null>(null);
  let failed = $state(false);

  onMount(async () => {
    try {
      info = await api.pinInfo();
    } catch {
      failed = true;
    }
  });

  function onPointerDown(e: PointerEvent) {
    // Drag the borderless window around. Ignore drags that begin on a button.
    if ((e.target as HTMLElement).closest('button')) return;
    void win.startDragging();
  }

  function onDoubleClick() {
    void win.close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') void win.close();
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="pin" onpointerdown={onPointerDown} ondblclick={onDoubleClick} role="presentation">
  {#if info}
    <img src={fileSrc(info.path)} alt="Pinned capture" draggable="false" />
  {:else if failed}
    <div class="error">Could not load the pinned capture.</div>
  {:else}
    <div class="error">Loading…</div>
  {/if}

  <button class="close" title="Close pin" onclick={() => void win.close()}>×</button>
</div>

<style>
  .pin {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: transparent;
    user-select: none;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    pointer-events: none;
    -webkit-user-drag: none;
  }
  .close {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 22px;
    height: 22px;
    padding: 0;
    font-size: 14px;
    line-height: 1;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.35);
    background: rgba(0, 0, 0, 0.35);
    color: #fff;
    opacity: 0;
    transition: opacity 0.12s ease;
    cursor: pointer;
  }
  .pin:hover .close {
    opacity: 1;
  }
  .error {
    display: grid;
    place-items: center;
    height: 100%;
    color: rgba(255, 255, 255, 0.75);
    font-size: 0.85rem;
    background: #222;
  }
</style>
