<script lang="ts">
  import SettingsView from './SettingsView.svelte';

  let { onClose, section = null }: { onClose: () => void; section?: string | null } = $props();
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
    <div class="dialog-head">
      <div class="dialog-title">
        <h2>Settings</h2>
        <p class="sub">Changes are written to your ShareV config file.</p>
      </div>
      <button class="dialog-close" onclick={onClose} title="Close (Esc)" aria-label="Close settings">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    </div>
    <div class="dialog-body">
      <SettingsView section={section} hideHeader />
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
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text-dim);
    cursor: pointer;
  }
  .dialog-close:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .dialog-body {
    overflow-y: auto;
    padding: 0 0.9rem 0.9rem;
  }
  .dialog-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    padding: 0.85rem 0.9rem 0.7rem;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.8rem;
  }
  .dialog-title h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .dialog-title .sub {
    margin: 0.15rem 0 0;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
</style>
