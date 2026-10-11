<script lang="ts">
  import type { CaptureTask } from '../types';

  let {
    tasks,
    available,
    onChange,
  }: {
    tasks: CaptureTask[];
    available: CaptureTask[];
    onChange: (tasks: CaptureTask[]) => void;
  } = $props();

  const labels: Record<CaptureTask, string> = {
    save: 'Save to disk',
    history: 'Add to history',
    copy: 'Copy image',
    editor: 'Open editor',
    open: 'Open file',
    upload: 'Upload',
    pin: 'Pin to screen',
    command: 'Run command',
    notify: 'Notify',
    copyLink: 'Copy link',
  };

  const descriptions: Partial<Record<CaptureTask, string>> = {
    save: 'Write the capture to the save directory.',
    history: 'Record it in the history list.',
    copy: 'Copy the image itself to the clipboard.',
    editor: 'Open the annotation editor (frontend).',
    open: 'Open the file with the default application.',
    upload: 'Send it to the configured uploader.',
    pin: 'Keep it on screen in a borderless window.',
    command: 'Run the after-capture shell command.',
    notify: 'Show a desktop notification.',
    copyLink: 'Copy the upload link to the clipboard.',
  };

  const remaining = $derived(available.filter((t) => !tasks.includes(t)));

  function add(task: CaptureTask) {
    onChange([...tasks, task]);
  }

  function remove(index: number) {
    onChange(tasks.filter((_, i) => i !== index));
  }

  function move(index: number, delta: number) {
    const next = [...tasks];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  }
</script>

<div class="chain">
  <ul class="chips">
    {#each tasks as task, i (task)}
      <li class="chip">
        <span class="chip-label">{labels[task]}</span>
        <span class="chip-actions">
          <button class="icon" title="Move up" disabled={i === 0} onclick={() => move(i, -1)}>↑</button>
          <button
            class="icon"
            title="Move down"
            disabled={i === tasks.length - 1}
            onclick={() => move(i, 1)}>↓</button>
          <button class="icon danger" title="Remove" onclick={() => remove(i)}>×</button>
        </span>
      </li>
    {/each}
    {#if tasks.length === 0}
      <li class="empty">No tasks — the capture is discarded.</li>
    {/if}
  </ul>

  {#if remaining.length > 0}
    <div class="add-row">
      <select
        onchange={(e) => {
          const value = (e.target as HTMLSelectElement).value as CaptureTask;
          if (value) add(value);
          (e.target as HTMLSelectElement).value = '';
        }}
      >
        <option value="">Add a task…</option>
        {#each remaining as task (task)}
          <option value={task}>{labels[task]}</option>
        {/each}
      </select>
    </div>
  {/if}

  {#each tasks as task (task)}
    {#if descriptions[task]}
      <p class="hint">{descriptions[task]}</p>
    {/if}
  {/each}
</div>

<style>
  .chain {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .chip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.35rem 0.55rem;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .chip-label {
    font-size: 0.85rem;
  }
  .chip-actions {
    display: flex;
    gap: 0.2rem;
  }
  .icon {
    padding: 0.1rem 0.4rem;
    font-size: 0.72rem;
    border-radius: 6px;
  }
  .icon.danger {
    color: var(--danger);
  }
  .empty {
    font-size: 0.82rem;
    color: var(--text-dim);
    padding: 0.4rem;
  }
  .add-row select {
    width: 100%;
  }
  .hint {
    margin: 0;
    font-size: 0.74rem;
    color: var(--text-dim);
  }
</style>
