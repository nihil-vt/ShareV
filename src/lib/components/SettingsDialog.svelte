<script lang="ts">
  import SettingsView from './SettingsView.svelte';

  let { onClose }: { onClose: () => void } = $props();
</script>

<div
  class="dialog-overlay"
  onclick={onClose}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
  role="button"
  tabindex="0"
  aria-label="Close settings"
>
  <div
    class="dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Settings"
    tabindex="-1"
  >
    <button class="dialog-close" onclick={onClose} title="Close (Esc)" aria-label="Close settings">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
    </button>
    <div class="dialog-body">
      <SettingsView />
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    padding: 1rem;
  }
  .dialog {
    position: relative;
    width: min(880px, 100%);
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 16px 64px rgba(0, 0, 0, 0.5);
    animation: pop 0.15s ease;
  }
  @keyframes pop {
    from { transform: scale(0.97); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
  .dialog-close {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text-dim);
    cursor: pointer;
    z-index: 2;
  }
  .dialog-close:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .dialog-body {
    overflow-y: auto;
    padding: 0.25rem;
  }
</style>
