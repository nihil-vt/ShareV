<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { app, loadAudioDevices, saveConfig, toast } from '../state.svelte';
  import type { Config } from '../types';

  const initial = structuredClone($state.snapshot(app.config)) as Config;
  let draft = $state<Config>(initial);

  onMount(() => {
    loadAudioDevices();
  });

  const systemDevices = $derived(app.audioDevices.filter((d) => d.monitor));
  const microphoneDevices = $derived(app.audioDevices.filter((d) => !d.monitor));

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
    </div>

    <div class="card">
      <h2>Behaviour</h2>

      <label class="check">
        <input type="checkbox" bind:checked={draft.copyToClipboard} />
        <span>Copy capture to clipboard</span>
      </label>

      <label class="check">
        <input type="checkbox" bind:checked={draft.openEditorAfterCapture} />
        <span>Open the annotation editor after each capture</span>
      </label>

      <label class="check">
        <input type="checkbox" bind:checked={draft.showNotification} />
        <span>Show a notification after each capture</span>
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
        <span>Register global hotkeys (X11 only)</span>
      </label>

      <label class="field">
        <span>Region capture</span>
        <input type="text" bind:value={draft.hotkeys.region} placeholder="PrintScreen" />
      </label>
      <label class="field">
        <span>Fullscreen capture</span>
        <input type="text" bind:value={draft.hotkeys.fullscreen} placeholder="Ctrl+PrintScreen" />
      </label>
      <label class="field">
        <span>Window capture</span>
        <input type="text" bind:value={draft.hotkeys.window} placeholder="Alt+PrintScreen" />
      </label>

      <p class="hint">
        On Wayland the compositor usually grabs keys. Bind the CLI instead, e.g. in Hyprland:<br />
        <code>bind = , PRINT, exec, sharev capture region</code>
      </p>
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
  .actions {
    display: flex;
    gap: 0.6rem;
    margin-top: 1.1rem;
  }
</style>
