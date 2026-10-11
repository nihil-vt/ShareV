<script lang="ts">
  import { app, type View } from '../state.svelte';

  const items: { id: View; label: string; icon: string }[] = [
    { id: 'capture', label: 'Capture', icon: '⛶' },
    { id: 'record', label: 'Record', icon: '⏺' },
    { id: 'history', label: 'History', icon: '☰' },
    { id: 'settings', label: 'Settings', icon: '⚙' },
    { id: 'diagnostics', label: 'Diagnostics', icon: '✚' },
  ];
</script>

<aside class="sidebar">
  <div class="brand">
    <span class="logo">S</span>
    <span class="name">ShareV</span>
  </div>

  <nav>
    {#each items as item (item.id)}
      <button class="nav" class:active={app.view === item.id} onclick={() => (app.view = item.id)}>
        <span class="icon">{item.icon}</span>
        <span>{item.label}</span>
        {#if item.id === 'history' && app.history.length > 0}
          <span class="badge">{app.history.length}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="spacer"></div>
  <div class="foot">v{app.diagnostics?.version ?? '0.1.0'}</div>
</aside>
