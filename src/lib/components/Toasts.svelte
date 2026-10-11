<script lang="ts">
  import { app } from '../state.svelte';
</script>

<div class="toasts">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">{t.text}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 50;
    max-width: 340px;
  }
  .toast {
    padding: 0.6rem 0.9rem;
    border-radius: 10px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 0.85rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    animation: slide-in 0.18s ease;
    word-break: break-word;
  }
  .toast.success {
    border-color: color-mix(in srgb, var(--success) 55%, var(--border));
  }
  .toast.error {
    border-color: color-mix(in srgb, var(--danger) 60%, var(--border));
  }
  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
