<script lang="ts">
  import { api } from '../api';
  import { abortRecording, app, stopRecording, toast } from '../state.svelte';

  let elapsed = $state('00:00');
  // Wall-clock pause gaps excluded, mirroring the backend duration math.
  let gapMs = 0;
  let pauseBegin = 0;

  $effect(() => {
    if (!app.recording) {
      gapMs = 0;
      pauseBegin = 0;
    } else if (app.recording.paused) {
      if (!pauseBegin) pauseBegin = Date.now();
    } else if (pauseBegin) {
      gapMs += Date.now() - pauseBegin;
      pauseBegin = 0;
    }
  });

  $effect(() => {
    const started = app.recordingStartedAt;
    if (!started) {
      elapsed = '00:00';
      return;
    }
    const tick = () => {
      const total = Math.max(0, Math.floor((Date.now() - started - gapMs) / 1000));
      const mins = Math.floor(total / 60);
      const secs = total % 60;
      elapsed = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    };
    tick();
    const id = setInterval(() => {
      if (!app.recording?.paused) tick();
    }, 500);
    return () => clearInterval(id);
  });

  async function stop() {
    try {
      await stopRecording();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function togglePause() {
    if (!app.recording) return;
    try {
      app.recording = await api.pauseRecording(!app.recording.paused);
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

{#if app.recording}
  <div class="recorder" class:paused={app.recording.paused} role="status" aria-live="polite" aria-label={app.recording.paused ? 'Recording paused' : 'Recording in progress'}>
    <span class="rec-dot"></span>
    <span class="rec-text">{app.recording.paused ? 'PAUSED' : 'REC'}</span>
    <span class="rec-time">{elapsed}</span>
    <span class="rec-meta">{app.recording.kind} · {app.recording.format.toUpperCase()}{#if app.config} · {app.config.record.fps} fps · audio {app.config.record.audio}{/if} · {app.recording.backend}</span>
    <button class="rec-pause" onclick={togglePause} title={app.recording.paused ? 'Resume recording' : 'Pause recording'}>
      {app.recording.paused ? '▶ Resume' : '⏸ Pause'}
    </button>
    <button class="rec-abort" onclick={() => abortRecording()} title="Discard the recording without saving">
      Abort
    </button>
    <button class="rec-stop" onclick={stop} title="Stop recording">
      <span class="stop-icon">■</span> Stop
    </button>
  </div>
{/if}

<style>
  .recorder {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.8rem;
    background: #2a1215;
    border-bottom: 1px solid var(--danger);
    font-size: 0.8rem;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .recorder.paused {
    background: #232027;
    border-bottom-color: var(--border);
  }
  .recorder.paused .rec-dot {
    animation: none;
    opacity: 0.5;
  }
  .rec-pause {
    padding: 0.3rem 0.8rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font-size: 0.78rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .rec-pause:hover {
    border-color: var(--accent);
  }
  .rec-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--danger);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.25; }
  }
  .rec-text {
    font-weight: 700;
    color: var(--danger);
    letter-spacing: 0.06em;
  }
  .rec-time {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-family: ui-monospace, monospace;
  }
  .rec-meta {
    color: var(--text-dim);
    font-size: 0.75rem;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rec-stop {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.8rem;
    border-radius: 7px;
    border: 1px solid var(--danger);
    background: var(--danger);
    color: #fff;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }
  .rec-stop:hover {
    background: var(--danger-2);
  }
  .rec-abort {
    padding: 0.3rem 0.8rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-dim);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .rec-abort:hover {
    color: var(--danger);
    border-color: var(--danger);
  }
  .stop-icon {
    font-size: 0.7rem;
    line-height: 1;
  }
</style>
