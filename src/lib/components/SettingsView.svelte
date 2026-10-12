<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { app, loadAudioDevices, saveConfig, toast } from '../state.svelte';
  import { checkForUpdates, initUpdates, installUpdate, updateProgress, updates } from '../updates.svelte';
  import TaskChainEditor from './TaskChainEditor.svelte';
  import type { CaptureTask, Config, EffectKind } from '../types';

  const initial = structuredClone($state.snapshot(app.config)) as Config;
  let draft = $state<Config>(initial);

  onMount(() => {
    loadAudioDevices();
    initUpdates();
  });

  const systemDevices = $derived(app.audioDevices.filter((d) => d.monitor));
  const microphoneDevices = $derived(app.audioDevices.filter((d) => !d.monitor));

  const captureTasks: CaptureTask[] = [
    'save',
    'history',
    'copy',
    'editor',
    'open',
    'upload',
    'pin',
    'command',
    'notify',
    'ocr',
  ];
  const uploadTasks: CaptureTask[] = ['copyLink', 'notify', 'open'];

  const hotkeyActions = [
    { value: 'capture-region', label: 'Capture region' },
    { value: 'capture-fullscreen', label: 'Capture fullscreen' },
    { value: 'capture-window', label: 'Capture window' },
    { value: 'capture-scrolling', label: 'Capture scrolling' },
    { value: 'record-toggle', label: 'Toggle recording' },
    { value: 'open-editor', label: 'Open editor' },
  ];

  const hotkeyBackends = [
    { value: 'auto', label: 'Automatic (portal on Wayland)' },
    { value: 'portal', label: 'XDG Portal (Wayland)' },
    { value: 'global-shortcut', label: 'Global Shortcut (X11)' },
    { value: 'none', label: 'None' },
  ];

  const effectKinds: { value: EffectKind; label: string }[] = [
    { value: 'shadow', label: 'Drop shadow' },
    { value: 'border', label: 'Border' },
    { value: 'canvasResize', label: 'Canvas padding' },
    { value: 'roundedCorners', label: 'Rounded corners' },
    { value: 'blur', label: 'Blur' },
    { value: 'watermark', label: 'Watermark' },
  ];

  const watermarkPositions = [
    'top-left',
    'top-right',
    'bottom-left',
    'bottom-right',
    'center',
  ];

  let ocrStatus = $state<{ available: boolean; languages: string[] } | null>(null);
  let checkingOcr = $state(false);

  async function checkOcr() {
    checkingOcr = true;
    try {
      ocrStatus = await api.ocrStatus();
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      checkingOcr = false;
    }
  }

  function hasEffect(kind: EffectKind): boolean {
    return draft.effects.effects.includes(kind);
  }

  function toggleEffect(kind: EffectKind) {
    const current = draft.effects.effects;
    const next = current.includes(kind)
      ? current.filter((k) => k !== kind)
      : [...current, kind];
    draft.effects = { ...draft.effects, effects: next };
  }

  async function browse() {
    try {
      const dir = await api.pickDirectory();
      if (dir) draft.saveDirectory = dir;
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function save() {
    await saveConfig(structuredClone($state.snapshot(draft)) as Config);
  }

  function reset() {
    draft = structuredClone($state.snapshot(app.config)) as Config;
  }

  function setAfterCapture(tasks: CaptureTask[]) {
    draft.afterCapture = { ...draft.afterCapture, tasks };
  }

  function setAfterUpload(tasks: CaptureTask[]) {
    draft.afterUpload = { ...draft.afterUpload, tasks };
  }

  function addBinding() {
    draft.hotkeys = {
      ...draft.hotkeys,
      bindings: [...draft.hotkeys.bindings, { action: 'capture-region', accelerator: '' }],
    };
  }

  function removeBinding(index: number) {
    draft.hotkeys = {
      ...draft.hotkeys,
      bindings: draft.hotkeys.bindings.filter((_, i) => i !== index),
    };
  }
</script>

<section class="view">
  <header class="view-head">
    <h1>Settings</h1>
    <p class="sub">Changes are written to your ShareV config file.</p>
  </header>

  <div class="cards">
    <div class="card">
      <h2>Output</h2>

      <label class="field">
        <span>Save directory</span>
        <div class="inline">
          <input type="text" bind:value={draft.saveDirectory} />
          <button class="ghost" onclick={browse}>Browse…</button>
        </div>
      </label>

      <label class="field">
        <span>Image format</span>
        <select bind:value={draft.imageFormat}>
          <option value="png">PNG (lossless)</option>
          <option value="jpeg">JPEG</option>
          <option value="webp">WebP</option>
        </select>
      </label>

      <label class="field">
        <span>Quality ({draft.jpegQuality})</span>
        <input type="range" min="1" max="100" bind:value={draft.jpegQuality} />
      </label>

      <label class="field">
        <span>Filename template (strftime)</span>
        <input type="text" bind:value={draft.filenameTemplate} placeholder="%Y-%m-%d_%H-%M-%S" />
      </label>

      <label class="field">
        <span>Capture delay (seconds)</span>
        <input type="number" min="0" max="60" bind:value={draft.captureDelaySeconds} />
      </label>

      <label class="field">
        <span>Target monitor (blank = focused)</span>
        <input type="text" bind:value={draft.monitor} placeholder="e.g. HDMI-1" />
      </label>

      <label class="field">
        <span>History limit</span>
        <input type="number" min="1" max="10000" bind:value={draft.maxHistory} />
      </label>

      <label class="field">
        <span>Capture backend</span>
        <select bind:value={draft.captureBackend}>
          <option value="auto">Automatic</option>
          <option value="wlroots">wlroots (grim + slurp)</option>
          <option value="x11">X11 (ImageMagick import)</option>
          <option value="portal">XDG Portal (not implemented)</option>
        </select>
      </label>
    </div>

    <div class="card">
      <h2>After capture</h2>
      <p class="hint">Run these steps in order once an image is grabbed.</p>

      <TaskChainEditor
        tasks={draft.afterCapture.tasks ?? []}
        available={captureTasks}
        onChange={setAfterCapture}
      />

      <label class="field">
        <span>Command (for the "Run command" task)</span>
        <input
          type="text"
          bind:value={draft.afterCapture.command}
          placeholder={'my-tool --file {file}'}
        />
      </label>
      <p class="hint">
        The file path replaces <code>{'{file}'}</code> (or is appended) and is available as
        <code>$SHAREV_FILE</code>.
      </p>
    </div>

    <div class="card">
      <h2>After upload</h2>
      <p class="hint">Run these steps once an upload succeeds.</p>

      <TaskChainEditor
        tasks={draft.afterUpload.tasks ?? []}
        available={uploadTasks}
        onChange={setAfterUpload}
      />
    </div>

    <div class="card">
      <h2>OCR</h2>
      <p class="hint">
        Recognise text with Tesseract. Add the "Run OCR" task above to copy the result to the
        clipboard after a capture.
      </p>

      <label class="field">
        <span>Language</span>
        <input type="text" bind:value={draft.ocrLanguage} placeholder="eng" />
      </label>

      <div class="inline">
        <button class="ghost" onclick={checkOcr} disabled={checkingOcr}>
          {checkingOcr ? 'Checking…' : 'Check Tesseract'}
        </button>
      </div>

      {#if ocrStatus}
        {#if ocrStatus.available}
          <p class="hint">
            Tesseract is installed. Languages:
            {ocrStatus.languages.length ? ocrStatus.languages.join(', ') : 'none detected'}
          </p>
        {:else}
          <p class="hint error">Tesseract is not installed. Install it to use OCR.</p>
        {/if}
      {/if}
    </div>

    <div class="card">
      <h2>Scrolling capture</h2>
      <p class="hint">
        A region is captured repeatedly while you scroll, then stitched into one tall image.
      </p>

      <label class="field">
        <span>Maximum frames</span>
        <input type="number" min="1" max="300" bind:value={draft.scroll.maxFrames} />
      </label>
      <label class="field">
        <span>Delay between frames ({draft.scroll.frameDelayMs} ms)</span>
        <input
          type="range"
          min="100"
          max="2000"
          step="50"
          bind:value={draft.scroll.frameDelayMs}
        />
      </label>
      <label class="field">
        <span>Stop similarity ({Math.round(draft.scroll.similarityThreshold * 100)}%)</span>
        <input
          type="range"
          min="0.8"
          max="1"
          step="0.01"
          bind:value={draft.scroll.similarityThreshold}
        />
      </label>
    </div>

    <div class="card">
      <h2>Image effects</h2>
      <p class="hint">Applied to the saved capture, in the order listed.</p>

      <div class="effects">
        {#each effectKinds as effect (effect.value)}
          <label class="check">
            <input
              type="checkbox"
              checked={hasEffect(effect.value)}
              onchange={() => toggleEffect(effect.value)}
            />
            <span>{effect.label}</span>
          </label>
        {/each}
      </div>

      {#if hasEffect('shadow')}
        <fieldset>
          <legend>Shadow</legend>
          <label class="field">
            <span>Offset X</span>
            <input type="number" bind:value={draft.effects.shadow.offsetX} />
          </label>
          <label class="field">
            <span>Offset Y</span>
            <input type="number" bind:value={draft.effects.shadow.offsetY} />
          </label>
          <label class="field">
            <span>Blur ({draft.effects.shadow.blurRadius})</span>
            <input type="range" min="0" max="40" step="1" bind:value={draft.effects.shadow.blurRadius} />
          </label>
          <label class="field">
            <span>Color</span>
            <input type="color" bind:value={draft.effects.shadow.color} />
          </label>
          <label class="field">
            <span>Opacity ({Math.round(draft.effects.shadow.opacity * 100)}%)</span>
            <input type="range" min="0" max="1" step="0.05" bind:value={draft.effects.shadow.opacity} />
          </label>
        </fieldset>
      {/if}

      {#if hasEffect('border')}
        <fieldset>
          <legend>Border</legend>
          <label class="field">
            <span>Width</span>
            <input type="number" min="1" max="100" bind:value={draft.effects.border.width} />
          </label>
          <label class="field">
            <span>Color</span>
            <input type="color" bind:value={draft.effects.border.color} />
          </label>
        </fieldset>
      {/if}

      {#if hasEffect('canvasResize')}
        <fieldset>
          <legend>Canvas padding</legend>
          <label class="field">
            <span>Padding</span>
            <input type="number" min="1" max="200" bind:value={draft.effects.canvasResize.padding} />
          </label>
          <label class="field">
            <span>Color</span>
            <input type="color" bind:value={draft.effects.canvasResize.color} />
          </label>
        </fieldset>
      {/if}

      {#if hasEffect('roundedCorners')}
        <fieldset>
          <legend>Rounded corners</legend>
          <label class="field">
            <span>Radius</span>
            <input type="number" min="1" max="200" bind:value={draft.effects.roundedCorners.radius} />
          </label>
        </fieldset>
      {/if}

      {#if hasEffect('blur')}
        <fieldset>
          <legend>Blur</legend>
          <label class="field">
            <span>Radius ({draft.effects.blur.radius})</span>
            <input type="range" min="0" max="40" step="0.5" bind:value={draft.effects.blur.radius} />
          </label>
        </fieldset>
      {/if}

      {#if hasEffect('watermark')}
        <fieldset>
          <legend>Watermark</legend>
          <label class="field">
            <span>Text</span>
            <input type="text" bind:value={draft.effects.watermark.text} />
          </label>
          <label class="field">
            <span>Font size</span>
            <input type="number" min="8" max="200" bind:value={draft.effects.watermark.fontSize} />
          </label>
          <label class="field">
            <span>Color</span>
            <input type="color" bind:value={draft.effects.watermark.color} />
          </label>
          <label class="field">
            <span>Opacity ({Math.round(draft.effects.watermark.opacity * 100)}%)</span>
            <input type="range" min="0" max="1" step="0.05" bind:value={draft.effects.watermark.opacity} />
          </label>
          <label class="field">
            <span>Position</span>
            <select bind:value={draft.effects.watermark.position}>
              {#each watermarkPositions as position (position)}
                <option value={position}>{position}</option>
              {/each}
            </select>
          </label>
        </fieldset>
      {/if}
    </div>

    <div class="card">
      <h2>Uploads</h2>

      <label class="field">
        <span>Destination</span>
        <select bind:value={draft.upload.uploader}>
          <option value="none">None (keep local)</option>
          <option value="imgur">Imgur (anonymous)</option>
          <option value="http">Custom HTTP endpoint</option>
          <option value="command">Custom command</option>
        </select>
      </label>

      <label class="field">
        <span>Copied link format</span>
        <select bind:value={draft.upload.copyFormat}>
          <option value="url">Plain URL</option>
          <option value="markdown">Markdown image</option>
          <option value="html">HTML img tag</option>
        </select>
      </label>

      {#if draft.upload.uploader === 'imgur'}
        <label class="field">
          <span>Imgur Client-ID</span>
          <input
            type="text"
            bind:value={draft.upload.imgurClientId}
            placeholder="paste your Client-ID"
          />
        </label>
        <p class="hint">
          Create a free anonymous application at
          <code>https://api.imgur.com/oauth2/addclient</code> and paste its Client-ID.
          Uploads stay anonymous.
        </p>
      {/if}

      {#if draft.upload.uploader === 'http'}
        <label class="field">
          <span>Endpoint URL</span>
          <input
            type="text"
            bind:value={draft.upload.httpUrl}
            placeholder="https://example.com/upload"
          />
        </label>
        <label class="field">
          <span>File field name</span>
          <input type="text" bind:value={draft.upload.httpField} placeholder="file" />
        </label>
        <label class="field">
          <span>Response URL path</span>
          <input
            type="text"
            bind:value={draft.upload.httpResponsePath}
            placeholder="data.link (blank = use raw body)"
          />
        </label>
        <label class="field">
          <span>Extra headers (one per line)</span>
          <textarea
            bind:value={draft.upload.httpHeaders}
            placeholder="Authorization: Bearer …"
            rows="3"
          ></textarea>
        </label>
      {/if}

      {#if draft.upload.uploader === 'command'}
        <label class="field">
          <span>Command</span>
          <input
            type="text"
            bind:value={draft.upload.command}
            placeholder={'my-uploader --file {file}'}
          />
        </label>
        <p class="hint">
          The file path is substituted for <code>{'{file}'}</code> (or appended). It is also
          available as <code>$SHAREV_FILE</code>. The last non-empty line of stdout is used as
          the URL.
        </p>
      {/if}
    </div>

    <div class="card">
      <h2>Recording</h2>

      <label class="field">
        <span>Format</span>
        <select bind:value={draft.record.format}>
          <option value="mp4">MP4 / H.264</option>
          <option value="webm">WebM / VP9</option>
          <option value="gif">Animated GIF</option>
        </select>
      </label>

      <label class="field">
        <span>Frame rate</span>
        <input type="number" min="1" max="120" bind:value={draft.record.fps} />
      </label>

      {#if draft.record.format === 'gif'}
        <label class="field">
          <span>GIF frame rate</span>
          <input type="number" min="1" max="60" bind:value={draft.record.gifFps} />
        </label>
      {/if}

      <label class="field">
        <span>Audio</span>
        <select bind:value={draft.record.audio}>
          <option value="none">None</option>
          <option value="system">System audio</option>
          <option value="microphone">Microphone</option>
          <option value="both">System + microphone</option>
        </select>
      </label>

      {#if draft.record.audio === 'system' || draft.record.audio === 'both'}
        <label class="field">
          <span>System audio device</span>
          <select bind:value={draft.record.systemDevice}>
            <option value="">Default monitor</option>
            {#each systemDevices as device (device.name)}
              <option value={device.name}>{device.description}</option>
            {/each}
          </select>
        </label>
      {/if}

      {#if draft.record.audio === 'microphone' || draft.record.audio === 'both'}
        <label class="field">
          <span>Microphone device</span>
          <select bind:value={draft.record.microphoneDevice}>
            <option value="">Default source</option>
            {#each microphoneDevices as device (device.name)}
              <option value={device.name}>{device.description}</option>
            {/each}
          </select>
        </label>
      {/if}

      {#if app.audioDevices.length === 0 && draft.record.audio !== 'none'}
        <p class="hint">
          No audio devices detected. Make sure <code>pactl</code> (PulseAudio / PipeWire) is
          available.
        </p>
      {/if}
    </div>

    <div class="card">
      <h2>Hotkeys</h2>
      <label class="check">
        <input type="checkbox" bind:checked={draft.hotkeys.enabled} />
        <span>Register global hotkeys</span>
      </label>

      <label class="field">
        <span>Backend</span>
        <select bind:value={draft.hotkeys.backend}>
          {#each hotkeyBackends as b (b.value)}
            <option value={b.value}>{b.label}</option>
          {/each}
        </select>
      </label>

      <div class="bindings">
        {#each draft.hotkeys.bindings as binding, i (i)}
          <div class="binding">
            <select bind:value={binding.action}>
              {#each hotkeyActions as a (a.value)}
                <option value={a.value}>{a.label}</option>
              {/each}
            </select>
            <input
              type="text"
              bind:value={binding.accelerator}
              placeholder="Ctrl+PrintScreen"
            />
            <button class="ghost" onclick={() => removeBinding(i)}>Remove</button>
          </div>
        {/each}
        <button class="ghost" onclick={addBinding}>Add binding</button>
      </div>

      <p class="hint">
        On Wayland the compositor usually grabs keys; the portal backend registers them for you.
        Alternatively bind the CLI, e.g. in Hyprland:<br />
        <code>bind = , PRINT, exec, sharev capture region</code>
      </p>
    </div>

    <div class="card">
      <h2>Updates</h2>
      <p class="hint">ShareV {updates.current || '…'}</p>

      {#if updates.flatpak}
        <p class="hint">
          Installed as a Flatpak — updates are handled by Flatpak/Flathub.
        </p>
      {:else}
        {#if updates.status === 'checking'}
          <p class="hint">Checking for updates…</p>
        {:else if updates.status === 'uptodate'}
          <p class="hint">You're up to date.</p>
        {:else if updates.status === 'available'}
          <p class="hint">Version <b>{updates.version}</b> is available.</p>
          {#if updates.notes}<pre class="notes">{updates.notes}</pre>{/if}
        {:else if updates.status === 'downloading'}
          <p class="hint">Downloading… {updateProgress()}%</p>
        {:else if updates.status === 'ready'}
          <p class="hint">Restarting to finish the update…</p>
        {:else if updates.status === 'error'}
          <p class="hint error">{updates.error}</p>
        {:else}
          <p class="hint">Check GitHub for a newer release.</p>
        {/if}

        <div class="update-actions">
          <button
            class="ghost"
            onclick={checkForUpdates}
            disabled={updates.checking || updates.status === 'downloading'}
          >
            {updates.status === 'checking' ? 'Checking…' : 'Check for updates'}
          </button>
          {#if updates.status === 'available'}
            <button class="primary" onclick={installUpdate}>Download &amp; install</button>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="actions">
    <button class="primary" onclick={save}>Save settings</button>
    <button class="ghost" onclick={reset}>Reset</button>
  </div>
</section>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
  }
  .card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 1.1rem 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .card h2 {
    margin: 0;
    font-size: 0.95rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-dim);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.85rem;
  }
  .field > span {
    color: var(--text-dim);
  }
  .inline {
    display: flex;
    gap: 0.5rem;
  }
  .inline input {
    flex: 1;
    min-width: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .effects {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.35rem 0.8rem;
  }
  fieldset {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin: 0;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface-3);
  }
  fieldset legend {
    padding: 0 0.4rem;
    font-size: 0.78rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dim);
    line-height: 1.5;
  }
  code {
    background: var(--surface-3);
    padding: 0.1rem 0.35rem;
    border-radius: 5px;
    font-size: 0.75rem;
  }
  .bindings {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .binding {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .binding select {
    flex: 1;
    min-width: 0;
  }
  .binding input {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 0.6rem;
    margin-top: 1.1rem;
  }
  .update-actions {
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .notes {
    margin: 0;
    padding: 0.6rem 0.7rem;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 0.76rem;
    line-height: 1.5;
    white-space: pre-wrap;
    max-height: 180px;
    overflow-y: auto;
  }
  .hint.error {
    color: var(--danger);
  }
</style>
