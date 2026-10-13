<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { api, fileSrc, formatBytes, formatDate } from '../api';
  import {
    app,
    doCapture,
    doScrollingCapture,
    startRecording,
    stopAutoCapture,
    uploadFile,
    uploadFolder,
    reloadHistory,
    toast,
  } from '../state.svelte';
  import type { CaptureResult, CaptureTask, Geometry, HistoryEntry, MonitorInfo, UploaderKind, WindowInfo } from '../types';
  import RecorderBar from './RecorderBar.svelte';
  import SettingsDialog from './SettingsDialog.svelte';
  import QuickTaskMenu from './QuickTaskMenu.svelte';
  import AutoCaptureDialog from './AutoCaptureDialog.svelte';
  import OcrDialog from './OcrDialog.svelte';
  import UploadExtraDialog from './UploadExtraDialog.svelte';
  import ColorPickerDialog from './ColorPickerDialog.svelte';
  import HashDialog from './HashDialog.svelte';
  import CombineDialog from './CombineDialog.svelte';
  import QrDialog from './QrDialog.svelte';
  import MetadataDialog from './MetadataDialog.svelte';
  import WatchDialog from './WatchDialog.svelte';
  import ShortenDialog from './ShortenDialog.svelte';

  const DEFAULT_CAPTURE_TASKS: CaptureTask[] = ['save', 'history', 'copy', 'editor', 'notify'];
  const DEFAULT_UPLOAD_TASKS: CaptureTask[] = ['copyLink', 'notify'];

  const captureTaskMeta: { id: CaptureTask; label: string }[] = [
    { id: 'save', label: 'Save image to file' },
    { id: 'history', label: 'Save to history' },
    { id: 'copy', label: 'Copy image to clipboard' },
    { id: 'editor', label: 'Annotate image' },
    { id: 'open', label: 'Open image' },
    { id: 'upload', label: 'Upload image to host' },
    { id: 'pin', label: 'Pin image to screen' },
    { id: 'command', label: 'Perform actions' },
    { id: 'notify', label: 'Show notification' },
    { id: 'ocr', label: 'Perform OCR' },
    { id: 'quickmenu', label: 'Show quick task menu' },
  ];

  const uploadTaskMeta: { id: CaptureTask; label: string }[] = [
    { id: 'copyLink', label: 'Copy URL to clipboard' },
    { id: 'shortenUrl', label: 'Shorten URL first' },
    { id: 'notify', label: 'Show notification' },
    { id: 'open', label: 'Open URL' },
  ];

  const uploaders: { id: UploaderKind; label: string }[] = [
    { id: 'none', label: 'None (local only)' },
    { id: 'imgur', label: 'Imgur' },
    { id: 'http', label: 'Custom HTTP' },
    { id: 'command', label: 'Custom command' },
  ];

  let showCaptureMenu = $state(false);
  let showUploadMenu = $state(false);
  let showToolsMenu = $state(false);
  let showMonitors = $state(false);
  let showSettings = $state(false);
  let settingsSection = $state<string | null>(null);
  let showWindows = $state(false);
  let windows = $state<WindowInfo[]>([]);
  let loadingWindows = $state(false);
  let monitors = $state<MonitorInfo[]>([]);
  let delay = $state(0);
  let monitor = $state('');
  let selectedId = $state<string | null>(null);
  let lastRegionGeom = $state<Geometry | null>(null);
  let quickResult = $state<CaptureResult | null>(null);
  let showAutoDialog = $state(false);
  let showOcrDialog = $state(false);
  let showUploadExtra = $state(false);
  let showColorPicker = $state(false);
  let showHash = $state(false);
  let showCombine = $state(false);
  let showQr = $state(false);
  let showMetadata = $state(false);
  let showWatch = $state(false);
  let showShorten = $state(false);
  let histView = $state<'list' | 'thumbs'>('list');

  const captureTasks = $derived<CaptureTask[]>(
    app.config?.afterCapture.tasks ?? DEFAULT_CAPTURE_TASKS,
  );
  const uploadTasks = $derived<CaptureTask[]>(
    app.config?.afterUpload.tasks ?? DEFAULT_UPLOAD_TASKS,
  );
  const ocrEnabled = $derived(app.config?.features.ocr ?? true);
  const visibleCaptureTasks = $derived(
    captureTaskMeta.filter((t) => t.id !== 'ocr' || ocrEnabled),
  );
  const selected = $derived<HistoryEntry | null>(
    app.history.find((e) => e.id === selectedId) ?? null,
  );

  onMount(async () => {
    try {
      monitors = await api.listMonitors();
    } catch {
      monitors = [];
    }
    delay = app.config?.captureDelaySeconds ?? 0;
    monitor = app.config?.monitor ?? '';
    if (app.history.length === 0) void reloadHistory();
  });

  // The quick task menu pops up after each capture when enabled (ShareX parity).
  onMount(() => {
    const unlisten = listen<CaptureResult>('capture-completed', (event) => {
      const tasks = app.config?.afterCapture.tasks ?? DEFAULT_CAPTURE_TASKS;
      if (tasks.includes('quickmenu')) quickResult = event.payload;
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  function closeMenus() {
    showCaptureMenu = false;
    showUploadMenu = false;
    showToolsMenu = false;
  }

  function openSettings(section: string | null) {
    settingsSection = section;
    showSettings = true;
  }

  async function refreshLastRegion() {
    try {
      lastRegionGeom = await api.lastRegion();
    } catch {
      lastRegionGeom = null;
    }
  }

  function runLastRegion() {
    closeMenus();
    void doCapture('last-region', { delaySeconds: delay });
  }

  function captureMonitor(name: string) {
    closeMenus();
    void doCapture('fullscreen', { delaySeconds: delay, monitor: name || undefined });
  }

  function runCapture(kind: 'region' | 'fullscreen') {
    closeMenus();
    void doCapture(kind, { delaySeconds: delay, monitor: monitor || undefined });
  }

  function runScrolling() {
    closeMenus();
    void doScrollingCapture({ delaySeconds: delay });
  }

  async function openWindowPicker() {
    closeMenus();
    showWindows = true;
    if (windows.length === 0) {
      loadingWindows = true;
      try {
        windows = await api.listWindows();
        if (windows.length === 0) toast('No windows found', 'info');
      } catch (err) {
        toast(String(err), 'error');
      } finally {
        loadingWindows = false;
      }
    }
  }

  function chooseWindow(w: WindowInfo) {
    showWindows = false;
    void doCapture('window', { windowId: w.id, delaySeconds: delay, monitor: monitor || undefined });
  }

  function toggleAuto() {
    closeMenus();
    if (app.autoCapture) void stopAutoCapture();
    else showAutoDialog = true;
  }

  function runRecord(kind: 'region' | 'fullscreen' | 'window') {
    closeMenus();
    void startRecording(kind, { delaySeconds: delay, monitor: monitor || undefined });
  }

  function uploadLast() {
    if (!app.last) return;
    void uploadFile(app.last.path);
  }

  async function uploadFileAction() {
    closeMenus();
    try {
      const picked = await api.pickFile();
      if (picked) await uploadFile(picked);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function copyLastLink() {
    closeMenus();
    if (!app.lastUpload) return;
    try {
      await api.copyText(app.lastUpload.link);
      toast('Link copied', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function uploadFolderAction() {
    closeMenus();
    try {
      const picked = await api.pickDirectory();
      if (picked) await uploadFolder(picked);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function ocrLast() {
    closeMenus();
    if (!app.last || app.last.kind === 'recording') return;
    if (!app.config?.features.ocr) {
      toast('Enable the OCR feature in Settings first', 'info');
      return;
    }
    try {
      const recognized = await api.ocrRecognize(app.last.path, app.config?.ocrLanguage ?? 'eng');
      if (!recognized.trim()) {
        toast('No text recognized', 'info');
        return;
      }
      await api.copyText(recognized);
      toast('OCR result copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function pinLast() {
    closeMenus();
    if (!app.last || app.last.kind === 'recording') return;
    try {
      await api.pinCapture(app.last.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function saveAsSelected() {
    if (!selected) return;
    try {
      const dest = await api.saveCaptureAs(selected.path);
      toast(`Saved to ${dest}`, 'success');
    } catch (err) {
      const message = String(err);
      if (!message.toLowerCase().includes('cancel')) toast(String(err), 'error');
    }
  }

  function openScreenshotsFolder() {
    const dir = app.config?.saveDirectory;
    if (!dir) {
      toast('No save directory configured', 'error');
      return;
    }
    api.revealPath(dir).catch((err: unknown) => toast(String(err), 'error'));
  }

  async function persistConfig() {
    if (!app.config) return;
    try {
      await api.saveConfig(structuredClone($state.snapshot(app.config)));
    } catch (err) {
      toast(`Could not save settings: ${err}`, 'error');
    }
  }

  function toggleCaptureTask(id: CaptureTask) {
    if (!app.config) return;
    const cur = app.config.afterCapture.tasks ?? [...DEFAULT_CAPTURE_TASKS];
    app.config.afterCapture.tasks = cur.includes(id)
      ? cur.filter((t) => t !== id)
      : [...cur, id];
    void persistConfig();
  }

  function toggleUploadTask(id: CaptureTask) {
    if (!app.config) return;
    const cur = app.config.afterUpload.tasks ?? [...DEFAULT_UPLOAD_TASKS];
    app.config.afterUpload.tasks = cur.includes(id)
      ? cur.filter((t) => t !== id)
      : [...cur, id];
    void persistConfig();
  }

  function setUploader(id: UploaderKind) {
    if (!app.config) return;
    app.config.upload.uploader = id;
    void persistConfig();
  }

  function selectEntry(id: string) {
    selectedId = selectedId === id ? null : id;
  }

  async function openEntry(e: HistoryEntry) {
    try {
      await api.openPath(e.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function copyEntry(e: HistoryEntry) {
    try {
      await api.copyFileToClipboard(e.path);
      toast('Copied to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function editEntry(e: HistoryEntry) {
    try {
      await api.openEditor(e.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function pinEntry(e: HistoryEntry) {
    try {
      await api.pinCapture(e.path);
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function deleteEntry(e: HistoryEntry) {
    try {
      await api.deleteHistory(e.id);
      if (selectedId === e.id) selectedId = null;
      await reloadHistory();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function clearHistory() {
    try {
      await api.clearHistory();
      selectedId = null;
      await reloadHistory();
    } catch (err) {
      toast(String(err), 'error');
    }
  }
</script>

<div class="main">
  <RecorderBar />

  <div class="body">
    <!-- Left action panel -->
    <aside class="side">
      <div class="capture-split">
        <button
          class="capture-main"
          disabled={app.busy}
          onclick={() => runCapture('region')}
          title="Capture region (PrntScr)"
        >
          <span class="big-icon">▣</span>
          <span class="big-label">Capture</span>
        </button>
        <button
          class="capture-caret"
          disabled={app.busy}
          onclick={() => { showCaptureMenu = !showCaptureMenu; if (showCaptureMenu) void refreshLastRegion(); }}
          aria-haspopup="menu"
          aria-expanded={showCaptureMenu}
          title="More capture options"
        >▾</button>
        {#if showCaptureMenu}
          <div class="menu" role="menu">
            <button role="menuitem" class="mi" disabled={app.busy} onclick={() => runCapture('fullscreen')}>
              <span class="mi-icon">⛶</span> Fullscreen
            </button>
            <button role="menuitem" class="mi" disabled={app.busy} onclick={openWindowPicker}>
              <span class="mi-icon">❐</span> Window…
            </button>
            <button role="menuitem" class="mi" disabled={app.busy} onclick={() => runCapture('region')}>
              <span class="mi-icon">▣</span> Region
            </button>
            <button role="menuitem" class="mi" disabled={app.busy || !lastRegionGeom} onclick={runLastRegion} title={lastRegionGeom ? `Last region (${lastRegionGeom.width}×${lastRegionGeom.height})` : 'Capture a region first'}>
              <span class="mi-icon">⧉</span> Last region{#if lastRegionGeom}<span class="mi-dims">{lastRegionGeom.width}×{lastRegionGeom.height}</span>{/if}
            </button>
            <button role="menuitem" class="mi" disabled={app.busy} onclick={runScrolling}>
              <span class="mi-icon">⇕</span> Scrolling capture
            </button>
            <button role="menuitem" class="mi" disabled={app.busy} onclick={() => (showMonitors = !showMonitors)}>
              <span class="mi-icon">🖥</span> Fullscreen on monitor…
            </button>
            {#if showMonitors}
              <button role="menuitem" class="mi sub" disabled={app.busy} onclick={() => captureMonitor('')}>
                Auto (focused)
              </button>
              {#each monitors as m (m.name)}
                <button role="menuitem" class="mi sub" disabled={app.busy} onclick={() => captureMonitor(m.name)}>
                  {m.focused ? '● ' : ''}{m.name} · {m.width}×{m.height}
                </button>
              {/each}
            {/if}
            <div class="mi-sep"></div>
            <button role="menuitem" class="mi" onclick={toggleAuto}>
              <span class="mi-icon">⟳</span>
              {app.autoCapture ? `Stop auto capture (${app.autoCapture.count})` : 'Auto capture…'}
            </button>
            <div class="mi-sep"></div>
            <div class="mi-head">Screen recording</div>
            <button role="menuitem" class="mi" disabled={!!app.recording} onclick={() => runRecord('region')}>
              <span class="mi-icon rec">⏺</span> Record region
            </button>
            <button role="menuitem" class="mi" disabled={!!app.recording} onclick={() => runRecord('fullscreen')}>
              <span class="mi-icon rec">⏺</span> Record fullscreen
            </button>
            <button role="menuitem" class="mi" disabled={!!app.recording} onclick={() => runRecord('window')}>
              <span class="mi-icon rec">⏺</span> Record window
            </button>
          </div>
        {/if}
      </div>
      {#if app.busy}<div class="working">working…</div>{/if}

      <div class="capture-split">
        <button class="capture-main upload-main" disabled={!app.last || app.uploading} onclick={uploadLast} title="Upload the last capture">
          <span class="big-icon">⬆</span>
          <span class="big-label">Upload</span>
        </button>
        <button
          class="capture-caret upload-caret"
          onclick={() => (showUploadMenu = !showUploadMenu)}
          aria-haspopup="menu"
          aria-expanded={showUploadMenu}
          title="More upload options"
        >▾</button>
        {#if showUploadMenu}
          <div class="menu" role="menu">
            <button role="menuitem" class="mi" disabled={!app.last || app.uploading} onclick={uploadLast}>
              <span class="mi-icon">⬆</span> Upload last capture
            </button>
            <button role="menuitem" class="mi" disabled={app.uploading} onclick={uploadFileAction}>
              <span class="mi-icon">🗀</span> Upload file…
            </button>
            <button role="menuitem" class="mi" disabled={app.uploading} onclick={uploadFolderAction}>
              <span class="mi-icon">🗁</span> Upload folder…
            </button>
            <button role="menuitem" class="mi" disabled={app.uploading} onclick={() => { closeMenus(); showUploadExtra = true; }}>
              <span class="mi-icon">🔤</span> Upload text / URL…
            </button>
            <button role="menuitem" class="mi" disabled={!app.lastUpload} onclick={copyLastLink}>
              <span class="mi-icon">🔗</span> Copy last link
            </button>
          </div>
        {/if}
      </div>

      <div class="opt-row">
        <label class="opt">
          <span>Delay</span>
          <select bind:value={delay}>
            <option value={0}>None</option>
            <option value={1}>1s</option>
            <option value={3}>3s</option>
            <option value={5}>5s</option>
            <option value={10}>10s</option>
          </select>
        </label>
        <label class="opt">
          <span>Monitor</span>
          <select bind:value={monitor}>
            <option value="">Auto</option>
            {#each monitors as m (m.name)}
              <option value={m.name}>{m.focused ? '● ' : ''}{m.name}</option>
            {/each}
          </select>
        </label>
      </div>

      <div class="sep"></div>

      <button class="side-btn" onclick={() => openSettings('sec-after-capture')}>
        <span class="mi-icon">⚙</span> Task settings…
      </button>
      <button class="side-btn" onclick={() => openSettings('sec-hotkeys')}>
        <span class="mi-icon">⌨</span> Hotkey settings…
      </button>
      <button class="side-btn" onclick={() => openSettings('sec-output')}>
        <span class="mi-icon">🛠</span> Application settings…
      </button>
      <div class="capture-split">
        <button class="capture-main tools-main" onclick={() => ocrLast()} disabled={!app.last || app.last.kind === 'recording'} title="Run OCR on the last capture">
          <span class="big-icon">🔧</span>
          <span class="big-label">Tools</span>
        </button>
        <button
          class="capture-caret tools-caret"
          onclick={() => (showToolsMenu = !showToolsMenu)}
          aria-haspopup="menu"
          aria-expanded={showToolsMenu}
          title="More tools"
        >▾</button>
        {#if showToolsMenu}
          <div class="menu" role="menu">
            <button role="menuitem" class="mi" disabled={!app.last || app.last.kind === 'recording'} onclick={ocrLast}>
              <span class="mi-icon">🔤</span> Run OCR on last capture
            </button>
            <button role="menuitem" class="mi" disabled={!app.last || app.last.kind === 'recording'} onclick={pinLast}>
              <span class="mi-icon">📌</span> Pin last capture to screen
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showOcrDialog = true; }}>
              <span class="mi-icon">🔤</span> OCR image…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showColorPicker = true; }}>
              <span class="mi-icon">🎨</span> Screen color picker…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showHash = true; }}>
              <span class="mi-icon">#</span> Hash checker…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showCombine = true; }}>
              <span class="mi-icon">⊞</span> Image combiner…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showQr = true; }}>
              <span class="mi-icon">◈</span> QR code…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showMetadata = true; }}>
              <span class="mi-icon">ⓘ</span> Image metadata…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showWatch = true; }}>
              <span class="mi-icon">👁</span> Watch folders…
            </button>
            <button role="menuitem" class="mi" onclick={() => { closeMenus(); showShorten = true; }}>
              <span class="mi-icon">→</span> Shorten URL…
            </button>
          </div>
        {/if}
      </div>
      <button class="side-btn" onclick={openScreenshotsFolder}>
        <span class="mi-icon">🗀</span> Screenshots folder…
      </button>

      <div class="sep"></div>

      <fieldset class="tasks">
        <legend>After capture</legend>
        {#each visibleCaptureTasks as t (t.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={captureTasks.includes(t.id)}
              onchange={() => toggleCaptureTask(t.id)}
            />
            <span>{t.label}</span>
          </label>
        {/each}
      </fieldset>

      <fieldset class="tasks">
        <legend>After upload</legend>
        {#each uploadTaskMeta as t (t.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={uploadTasks.includes(t.id)}
              onchange={() => toggleUploadTask(t.id)}
            />
            <span>{t.label}</span>
          </label>
        {/each}
      </fieldset>

      <label class="dest">
        <span>Image uploader</span>
        <select value={app.config?.upload.uploader ?? 'none'} onchange={(e) => setUploader(e.currentTarget.value as UploaderKind)}>
          {#each uploaders as u (u.id)}
            <option value={u.id}>{u.label}</option>
          {/each}
        </select>
      </label>
    </aside>

    <!-- Right: history + preview -->
    <section class="content">
      <div class="hist-head">
        <h2>History</h2>
        <span class="count">{app.history.length} item(s)</span>
        <div class="view-toggle" role="group" aria-label="History view">
          <button class:active={histView === 'list'} onclick={() => (histView = 'list')} title="List view">☰</button>
          <button class:active={histView === 'thumbs'} onclick={() => (histView = 'thumbs')} title="Thumbnail view">▦</button>
        </div>
        <button class="mini" disabled={app.history.length === 0} onclick={clearHistory}>Clear</button>
      </div>

      <div class="hist-list" role="listbox" aria-label="Capture history">
        {#if app.history.length === 0}
          <div class="empty">No captures yet — press <kbd>PrntScr</kbd> or use Capture.</div>
        {:else if histView === 'thumbs'}
          <div class="thumb-grid">
            {#each app.history as e (e.id)}
              <button
                class="thumb-card"
                class:selected={selectedId === e.id}
                onclick={() => selectEntry(e.id)}
                ondblclick={() => openEntry(e)}
                title={e.fileName}
                aria-label={e.fileName}
              >
                {#if e.kind === 'recording'}
                  <span class="thumb-big vid">▶</span>
                {:else}
                  <img class="thumb-big" src={fileSrc(e.path)} alt="" loading="lazy" />
                {/if}
                <span class="thumb-name">{e.fileName}</span>
              </button>
            {/each}
          </div>
        {:else}
          {#each app.history as e (e.id)}
            <button
              class="row"
              class:selected={selectedId === e.id}
              onclick={() => selectEntry(e.id)}
              ondblclick={() => openEntry(e)}
              role="option"
              aria-selected={selectedId === e.id}
            >
              {#if e.kind === 'recording'}
                <span class="thumb vid">▶</span>
              {:else}
                <img class="thumb" src={fileSrc(e.path)} alt="" loading="lazy" />
              {/if}
              <span class="r-name">{e.fileName}</span>
              <span class="r-size">{formatBytes(e.bytes)}</span>
              <span class="r-date">{formatDate(e.createdAt)}</span>
              {#if e.url}<span class="r-url" title={e.url}>🔗</span>{/if}
            </button>
          {/each}
        {/if}
      </div>

      {#if selected}
        <div class="preview">
          <div class="pv-media">
            {#if selected.kind === 'recording'}
              <!-- svelte-ignore a11y_media_has_caption -->
              <video src={fileSrc(selected.path)} controls></video>
            {:else}
              <img src={fileSrc(selected.path)} alt={selected.fileName} />
            {/if}
          </div>
          <div class="pv-side">
            <div class="pv-name">{selected.fileName}</div>
            <div class="pv-meta">{selected.width}×{selected.height} · {formatBytes(selected.bytes)} · {selected.backend}</div>
            {#if selected.url}<div class="pv-url">{selected.url}</div>{/if}
            <div class="pv-actions">
              <button onclick={() => selected && openEntry(selected)}>Open</button>
              <button onclick={() => selected && api.revealPath(selected.path).catch((err: unknown) => toast(String(err), 'error'))}>Folder</button>
              <button onclick={() => selected && copyEntry(selected)}>Copy</button>
              {#if selected.kind !== 'recording'}
                <button onclick={() => selected && editEntry(selected)}>Edit</button>
                <button onclick={() => selected && pinEntry(selected)}>Pin</button>
              {/if}
              <button onclick={() => selected && uploadFile(selected.path)} disabled={app.uploading}>Upload</button>
              <button onclick={saveAsSelected}>Save as…</button>
              <button class="danger" onclick={() => selected && deleteEntry(selected)}>Delete</button>
            </div>
          </div>
        </div>
      {/if}

      <div class="statusbar">
        <span>{app.history.length} item(s)</span>
        {#if app.autoCapture}
          <span class="st-auto">⟳ auto: {app.autoCapture.kind} ×{app.autoCapture.count}</span>
        {/if}
        {#if app.last}
          <span class="st-last">last: {app.last.fileName}</span>
        {/if}
      </div>
    </section>
  </div>

  {#if showWindows}
    <div
      class="overlay"
      onclick={() => (showWindows = false)}
      onkeydown={(e) => e.key === 'Escape' && (showWindows = false)}
      role="button"
      tabindex="0"
      aria-label="Close window picker"
    >
      <div
        class="picker"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Capture a window"
        tabindex="-1"
      >
        <div class="picker-head">
          <h2>Capture a window</h2>
          <button class="mini" onclick={() => (showWindows = false)}>Close</button>
        </div>
        {#if windows.length === 0}
          <div class="empty">{loadingWindows ? 'Loading windows…' : 'No windows found.'}</div>
        {:else}
          <ul class="window-list">
            {#each windows as w (w.id)}
              <li>
                <button class="window-item" onclick={() => chooseWindow(w)}>
                  <span class="window-title">{w.title || '(untitled)'}</span>
                  <span class="window-app">{w.app || '—'} · {w.width}×{w.height}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {/if}

  {#if showSettings}
    <SettingsDialog section={settingsSection} onClose={() => (showSettings = false)} />
  {/if}

  {#if quickResult}
    <QuickTaskMenu result={quickResult} onClose={() => (quickResult = null)} />
  {/if}

  {#if showAutoDialog}
    <AutoCaptureDialog onClose={() => (showAutoDialog = false)} />
  {/if}

  {#if showOcrDialog}
    <OcrDialog onClose={() => (showOcrDialog = false)} />
  {/if}

  {#if showUploadExtra}
    <UploadExtraDialog onClose={() => (showUploadExtra = false)} />
  {/if}

  {#if showColorPicker}
    <ColorPickerDialog onClose={() => (showColorPicker = false)} />
  {/if}

  {#if showHash}
    <HashDialog onClose={() => (showHash = false)} />
  {/if}

  {#if showCombine}
    <CombineDialog onClose={() => (showCombine = false)} />
  {/if}

  {#if showQr}
    <QrDialog onClose={() => (showQr = false)} />
  {/if}

  {#if showMetadata}
    <MetadataDialog onClose={() => (showMetadata = false)} />
  {/if}

  {#if showWatch}
    <WatchDialog onClose={() => (showWatch = false)} />
  {/if}

  {#if showShorten}
    <ShortenDialog onClose={() => (showShorten = false)} />
  {/if}
</div>

<style>
  .main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--surface);
    color: var(--text);
  }
  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* ---- Left panel ---- */
  .side {
    width: 236px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.7rem;
    border-right: 1px solid var(--border);
    background: var(--surface);
    overflow-y: auto;
  }
  .capture-split {
    position: relative;
    display: flex;
  }
  .capture-main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.6rem 0.8rem;
    border-radius: 9px 0 0 9px;
    border: 1px solid var(--accent);
    border-right: none;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }
  .capture-main:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 26%, transparent);
  }
  .capture-main:disabled {
    opacity: 0.55;
    cursor: progress;
  }
  .big-icon {
    font-size: 1.25rem;
    color: var(--accent);
  }
  .capture-caret {
    padding: 0 0.6rem;
    border-radius: 0 9px 9px 0;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--text);
    cursor: pointer;
    font-size: 0.8rem;
  }
  .capture-caret:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 26%, transparent);
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 10px 36px rgba(0, 0, 0, 0.45);
    padding: 4px;
    z-index: 100;
  }
  .mi {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    width: 100%;
    padding: 0.42rem 0.6rem;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mi:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .mi:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .mi-icon {
    width: 1.2rem;
    flex-shrink: 0;
    text-align: center;
    color: var(--text-dim);
  }
  .mi-icon.rec {
    color: var(--danger);
  }
  .mi-sep {
    height: 1px;
    background: var(--border);
    margin: 4px 6px;
  }
  .mi-head {
    font-size: 0.62rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    padding: 0.3rem 0.6rem 0.15rem;
  }
  .mi.sub {
    padding-left: 1.9rem;
    font-size: 0.76rem;
  }
  .mi-dims {
    margin-left: auto;
    font-size: 0.68rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .upload-main {
    border-color: var(--border);
    background: var(--surface-2);
  }
  .upload-main:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .upload-main .big-icon {
    color: var(--text-dim);
  }
  .upload-caret {
    border-color: var(--border);
    background: var(--surface-2);
  }
  .upload-caret:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .tools-main {
    border-color: var(--border);
    background: var(--surface-2);
  }
  .tools-main:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .tools-main .big-icon {
    color: var(--text-dim);
  }
  .tools-caret {
    border-color: var(--border);
    background: var(--surface-2);
  }
  .tools-caret:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .working {
    font-size: 0.72rem;
    color: var(--accent);
  }
  .side-btn {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.45rem 0.65rem;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.8rem;
    cursor: pointer;
    text-align: left;
  }
  .side-btn:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .side-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .opt-row {
    display: flex;
    gap: 0.5rem;
  }
  .opt {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: var(--text-dim);
  }
  .opt select,
  .dest select {
    padding: 0.32rem 0.4rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.78rem;
    min-width: 0;
  }
  .sep {
    height: 1px;
    background: var(--border);
  }
  .tasks {
    margin: 0;
    padding: 0.5rem 0.6rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 9px;
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
  }
  .tasks legend {
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--text-dim);
    padding: 0 0.3rem;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--accent);
  }
  .dest {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: var(--text-dim);
  }

  /* ---- Right content ---- */
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface-2);
  }
  .hist-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.55rem 0.9rem;
    border-bottom: 1px solid var(--border);
  }
  .hist-head h2 {
    margin: 0;
    font-size: 0.9rem;
  }
  .count {
    font-size: 0.72rem;
    color: var(--text-dim);
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
  .view-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 7px;
    overflow: hidden;
    margin-left: auto;
  }
  .view-toggle button {
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
  }
  .view-toggle button.active {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .hist-head .mini:last-child {
    margin-left: 0;
  }
  .mini:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--accent);
  }
  .mini:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .hist-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
  }
  .empty {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--text-dim);
    font-size: 0.82rem;
  }
  kbd {
    padding: 0.05rem 0.3rem;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.7rem;
    font-family: ui-monospace, monospace;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.3rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font-size: 0.78rem;
    cursor: pointer;
    text-align: left;
  }
  .row:hover {
    background: var(--surface-3);
  }
  .row.selected {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border-color: var(--accent);
  }
  .thumb {
    width: 44px;
    height: 30px;
    object-fit: cover;
    border-radius: 5px;
    border: 1px solid var(--border);
    background: var(--surface-3);
    flex-shrink: 0;
  }
  .thumb.vid {
    display: grid;
    place-items: center;
    font-size: 0.7rem;
    color: var(--accent);
  }
  .r-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .r-size,
  .r-date {
    color: var(--text-dim);
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .r-url {
    font-size: 0.75rem;
  }
  .thumb-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 0.5rem;
    align-content: start;
  }
  .thumb-card {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.4rem;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface);
    cursor: pointer;
  }
  .thumb-card:hover {
    border-color: var(--accent);
  }
  .thumb-card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .thumb-big {
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    border-radius: 6px;
    background: #0d0f14;
  }
  span.thumb-big.vid {
    display: grid;
    place-items: center;
    color: var(--accent);
    font-size: 1.2rem;
  }
  .thumb-name {
    font-size: 0.68rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ---- Preview ---- */
  .preview {
    display: flex;
    gap: 0.9rem;
    padding: 0.7rem 0.9rem;
    border-top: 1px solid var(--border);
    background: var(--surface);
    max-height: 220px;
  }
  .pv-media {
    flex-shrink: 0;
  }
  .pv-media img,
  .pv-media video {
    max-width: 280px;
    max-height: 170px;
    object-fit: contain;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: #0d0f14;
  }
  .pv-side {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .pv-name {
    font-weight: 600;
    font-size: 0.85rem;
    word-break: break-all;
  }
  .pv-meta {
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  .pv-url {
    font-size: 0.7rem;
    color: var(--accent);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pv-actions {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-top: auto;
  }
  .pv-actions button {
    padding: 0.32rem 0.7rem;
    font-size: 0.76rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .pv-actions button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  .pv-actions button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pv-actions button.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  /* ---- Status bar ---- */
  .statusbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.28rem 0.9rem;
    border-top: 1px solid var(--border);
    background: var(--surface);
    font-size: 0.7rem;
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .st-auto {
    color: var(--accent);
  }
  .st-last {
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ---- Window picker ---- */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .picker {
    width: min(680px, 92vw);
    max-height: 80vh;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
  }
  .picker-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.7rem;
  }
  .picker-head h2 {
    margin: 0;
    font-size: 0.92rem;
  }
  .window-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 0.5rem;
  }
  .window-item {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    width: 100%;
    text-align: left;
    padding: 0.5rem 0.7rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    cursor: pointer;
  }
  .window-item:hover {
    border-color: var(--accent);
  }
  .window-title {
    font-weight: 600;
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .window-app {
    font-size: 0.68rem;
    color: var(--text-dim);
  }
</style>
