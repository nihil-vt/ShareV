<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../api';
  import { app, reloadHistory, toast, uploadFile } from '../state.svelte';

  type Tool = 'rect' | 'ellipse' | 'arrow' | 'pen' | 'text' | 'pixelate' | 'crop';

  const tools: { id: Tool; label: string; icon: string }[] = [
    { id: 'rect', label: 'Rectangle', icon: '▭' },
    { id: 'ellipse', label: 'Ellipse', icon: '◯' },
    { id: 'arrow', label: 'Arrow', icon: '➔' },
    { id: 'pen', label: 'Pen', icon: '✎' },
    { id: 'text', label: 'Text', icon: 'T' },
    { id: 'pixelate', label: 'Pixelate', icon: '▦' },
    { id: 'crop', label: 'Crop', icon: '⛶' },
  ];

  const MAX_UNDO = 20;
  const MIN_CROP = 6;

  let canvas = $state<HTMLCanvasElement | null>(null);
  let tool = $state<Tool>('rect');
  let color = $state('#ff3b30');
  let strokeWidth = $state(4);
  let text = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let dirty = $state(false);
  let hasUndo = $state(false);
  let hasRedo = $state(false);
  let dims = $state({ w: 0, h: 0 });

  // Undo/redo snapshots hold the *before* state of every edit.
  let undoStack: ImageData[] = [];
  let redoStack: ImageData[] = [];
  let original: ImageData | null = null;

  // Transient drag state.
  let drawing = false;
  let startX = 0;
  let startY = 0;
  let base: ImageData | null = null;

  function ctx(): CanvasRenderingContext2D | null {
    return canvas?.getContext('2d', { willReadFrequently: true }) ?? null;
  }

  function sync() {
    hasUndo = undoStack.length > 0;
    hasRedo = redoStack.length > 0;
    if (canvas) dims = { w: canvas.width, h: canvas.height };
  }

  async function load() {
    const editing = app.editing;
    if (!editing || !canvas) return;
    loading = true;
    try {
      const dataUrl = await api.readImageBase64(editing.path);
      const image = new Image();
      await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error('could not decode the image'));
        image.src = dataUrl;
      });

      const c = canvas;
      c.width = image.naturalWidth || image.width;
      c.height = image.naturalHeight || image.height;
      const context = ctx();
      if (!context) throw new Error('canvas is unavailable');
      context.imageSmoothingEnabled = true;
      context.drawImage(image, 0, 0);

      original = context.getImageData(0, 0, c.width, c.height);
      undoStack = [];
      redoStack = [];
      dirty = false;
      dims = { w: c.width, h: c.height };
      sync();
    } catch (err) {
      toast(`Could not open the editor: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }

  function point(e: PointerEvent) {
    const c = canvas!;
    const rect = c.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (c.width / rect.width),
      y: (e.clientY - rect.top) * (c.height / rect.height),
    };
  }

  function commit() {
    if (!base) return;
    undoStack.push(base);
    if (undoStack.length > MAX_UNDO) undoStack.shift();
    redoStack = [];
    base = null;
    dirty = true;
    sync();
  }

  function onDown(e: PointerEvent) {
    if (loading || saving || !canvas) return;
    const context = ctx();
    if (!context) return;
    const p = point(e);

    drawing = true;
    startX = p.x;
    startY = p.y;
    base = context.getImageData(0, 0, canvas.width, canvas.height);
    canvas.setPointerCapture(e.pointerId);

    if (tool === 'pen') {
      context.beginPath();
      context.moveTo(p.x, p.y);
      context.strokeStyle = color;
      context.lineWidth = strokeWidth;
      context.lineCap = 'round';
      context.lineJoin = 'round';
    }
  }

  function onMove(e: PointerEvent) {
    if (!drawing || !canvas) return;
    const context = ctx();
    if (!context) return;
    const p = point(e);

    if (tool === 'pen') {
      context.lineTo(p.x, p.y);
      context.stroke();
      return;
    }

    if (base) context.putImageData(base, 0, 0);
    drawPreview(context, p.x, p.y);
  }

  function onUp(e: PointerEvent) {
    if (!drawing || !canvas) return;
    const context = ctx();
    if (!context) return;
    const p = point(e);
    drawing = false;
    try {
      canvas.releasePointerCapture(e.pointerId);
    } catch {
      /* pointer already released */
    }

    if (tool === 'pen') {
      commit();
      return;
    }

    if (base) context.putImageData(base, 0, 0);

    switch (tool) {
      case 'rect':
      case 'ellipse':
      case 'arrow':
      case 'pixelate':
        drawPreview(context, p.x, p.y);
        commit();
        break;
      case 'text':
        if (!text.trim()) {
          toast('Type some text first', 'info');
          base = null;
          break;
        }
        drawText(context, p.x, p.y);
        commit();
        break;
      case 'crop':
        applyCrop(context, p.x, p.y);
        break;
    }
  }

  function drawPreview(context: CanvasRenderingContext2D, x: number, y: number) {
    const x0 = Math.min(startX, x);
    const y0 = Math.min(startY, y);
    const w = Math.abs(x - startX);
    const h = Math.abs(y - startY);

    context.lineWidth = strokeWidth;
    context.strokeStyle = color;
    context.fillStyle = color;

    switch (tool) {
      case 'rect':
        context.beginPath();
        context.rect(x0, y0, w, h);
        context.stroke();
        break;
      case 'ellipse':
        context.beginPath();
        context.ellipse(x0 + w / 2, y0 + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
        context.stroke();
        break;
      case 'arrow':
        drawArrow(context, startX, startY, x, y);
        break;
      case 'pixelate':
        applyPixelate(context, startX, startY, x, y);
        break;
      case 'crop':
        drawCropOverlay(context, x0, y0, w, h);
        break;
    }
  }

  function drawArrow(
    context: CanvasRenderingContext2D,
    x0: number,
    y0: number,
    x1: number,
    y1: number,
  ) {
    const head = Math.max(10, strokeWidth * 3.5);
    const angle = Math.atan2(y1 - y0, x1 - x0);

    context.lineWidth = strokeWidth;
    context.strokeStyle = color;
    context.fillStyle = color;
    context.lineCap = 'round';

    context.beginPath();
    context.moveTo(x0, y0);
    context.lineTo(x1, y1);
    context.stroke();

    context.beginPath();
    context.moveTo(x1, y1);
    context.lineTo(
      x1 - head * Math.cos(angle - Math.PI / 6),
      y1 - head * Math.sin(angle - Math.PI / 6),
    );
    context.lineTo(
      x1 - head * Math.cos(angle + Math.PI / 6),
      y1 - head * Math.sin(angle + Math.PI / 6),
    );
    context.closePath();
    context.fill();
  }

  function drawText(context: CanvasRenderingContext2D, x: number, y: number) {
    const value = text.trim();
    if (!value) {
      toast('Type some text first', 'info');
      return;
    }
    const size = Math.max(16, strokeWidth * 6);
    context.font = `600 ${size}px system-ui, -apple-system, sans-serif`;
    context.textBaseline = 'top';
    context.lineWidth = Math.max(2, size / 8);
    context.strokeStyle = 'rgba(0, 0, 0, 0.85)';
    context.fillStyle = color;
    context.strokeText(value, x, y);
    context.fillText(value, x, y);
  }

  function region(x0: number, y0: number, x1: number, y1: number) {
    const c = canvas!;
    const x = Math.max(0, Math.floor(Math.min(x0, x1)));
    const y = Math.max(0, Math.floor(Math.min(y0, y1)));
    const w = Math.min(c.width - x, Math.floor(Math.abs(x1 - x0)));
    const h = Math.min(c.height - y, Math.floor(Math.abs(y1 - y0)));
    return { x, y, w, h };
  }

  function applyPixelate(
    context: CanvasRenderingContext2D,
    x0: number,
    y0: number,
    x1: number,
    y1: number,
  ) {
    const { x, y, w, h } = region(x0, y0, x1, y1);
    if (w < 2 || h < 2) return;

    const block = Math.max(6, Math.round(strokeWidth * 3));
    const small = document.createElement('canvas');
    small.width = Math.max(1, Math.floor(w / block));
    small.height = Math.max(1, Math.floor(h / block));
    const sctx = small.getContext('2d')!;
    sctx.imageSmoothingEnabled = false;
    sctx.drawImage(canvas!, x, y, w, h, 0, 0, small.width, small.height);

    context.save();
    context.imageSmoothingEnabled = false;
    context.drawImage(small, 0, 0, small.width, small.height, x, y, w, h);
    context.restore();
  }

  function drawCropOverlay(
    context: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
  ) {
    const c = canvas!;
    context.save();
    context.fillStyle = 'rgba(0, 0, 0, 0.55)';
    context.beginPath();
    context.rect(0, 0, c.width, c.height);
    context.rect(x, y, w, h);
    context.fill('evenodd');
    context.strokeStyle = color;
    context.lineWidth = Math.max(2, strokeWidth);
    context.strokeRect(x, y, w, h);
    context.restore();
  }

  function applyCrop(context: CanvasRenderingContext2D, x1: number, y1: number) {
    const { x, y, w, h } = region(startX, startY, x1, y1);
    if (w < MIN_CROP || h < MIN_CROP) {
      base = null;
      return;
    }

    const cropped = context.getImageData(x, y, w, h);
    canvas!.width = w;
    canvas!.height = h;
    ctx()!.putImageData(cropped, 0, 0);
    commit();
  }

  function undo() {
    if (!canvas || undoStack.length === 0) return;
    const context = ctx();
    if (!context) return;
    redoStack.push(context.getImageData(0, 0, canvas.width, canvas.height));
    const prev = undoStack.pop()!;
    if (canvas.width !== prev.width || canvas.height !== prev.height) {
      canvas.width = prev.width;
      canvas.height = prev.height;
    }
    ctx()!.putImageData(prev, 0, 0);
    dirty = true;
    sync();
  }

  function redo() {
    if (!canvas || redoStack.length === 0) return;
    const context = ctx();
    if (!context) return;
    undoStack.push(context.getImageData(0, 0, canvas.width, canvas.height));
    const next = redoStack.pop()!;
    if (canvas.width !== next.width || canvas.height !== next.height) {
      canvas.width = next.width;
      canvas.height = next.height;
    }
    ctx()!.putImageData(next, 0, 0);
    dirty = true;
    sync();
  }

  function revert() {
    if (!canvas || !original) return;
    const context = ctx();
    if (!context) return;
    undoStack.push(context.getImageData(0, 0, canvas.width, canvas.height));
    if (undoStack.length > MAX_UNDO) undoStack.shift();
    redoStack = [];
    if (canvas.width !== original.width || canvas.height !== original.height) {
      canvas.width = original.width;
      canvas.height = original.height;
    }
    ctx()!.putImageData(original, 0, 0);
    dirty = true;
    sync();
  }

  async function copy() {
    if (!canvas) return;
    try {
      await api.copyEdits(canvas.toDataURL('image/png'));
      toast('Copied annotated image to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function save() {
    const editing = app.editing;
    if (!editing || !canvas || saving) return;
    saving = true;
    try {
      const data = canvas.toDataURL('image/png');
      const result = await api.applyEdits(editing.path, data);
      app.last = result;
      await reloadHistory();
      toast(`Saved ${result.fileName}`, 'success');
      finish();
    } catch (err) {
      toast(`Could not save: ${err}`, 'error');
    } finally {
      saving = false;
    }
  }

  function close() {
    if (dirty && !confirm('Discard your edits? The original capture is already saved.')) return;
    finish();
  }

  /** Save the annotations, close the editor, then upload the result. */
  async function saveAndUpload() {
    const editing = app.editing;
    if (!editing || !canvas || saving) return;
    saving = true;
    try {
      const data = canvas.toDataURL('image/png');
      const result = await api.applyEdits(editing.path, data);
      app.last = result;
      await reloadHistory();
      finish();
      await uploadFile(result.path);
    } catch (err) {
      toast(`Could not save: ${err}`, 'error');
    } finally {
      saving = false;
    }
  }

  function finish() {
    app.editing = null;
    app.view = 'capture';
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const typing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA');
    const mod = e.ctrlKey || e.metaKey;

    if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (mod && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) {
      e.preventDefault();
      redo();
    } else if (e.key === 'Escape' && !typing) {
      close();
    }
  }

  onMount(() => {
    load();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

{#if app.editing}
  <section class="view editor">
    <header class="view-head row">
      <div>
        <h1>Annotate</h1>
        <p class="sub">
          {app.editing.fileName}
          {#if !loading && dims.w > 0}· {dims.w} × {dims.h}{/if}
        </p>
      </div>
      <div class="head-actions">
        <button class="ghost" onclick={close}>Close</button>
        <button onclick={copy} disabled={loading || saving}>Copy</button>
        <button onclick={saveAndUpload} disabled={loading || saving || app.config?.upload.uploader === 'none'}>
          Save &amp; Upload
        </button>
        <button class="primary" onclick={save} disabled={loading || saving}>
          {saving ? 'Saving…' : 'Save'}
        </button>
      </div>
    </header>

    <div class="toolbar">
      <div class="tool-group">
        {#each tools as item (item.id)}
          <button
            class="tool"
            class:active={tool === item.id}
            title={item.label}
            onclick={() => (tool = item.id)}
          >
            <span class="tool-icon">{item.icon}</span>
            <span class="tool-label">{item.label}</span>
          </button>
        {/each}
      </div>

      <div class="tool-group options">
        <label class="opt">
          <span>Color</span>
          <input type="color" bind:value={color} />
        </label>
        <label class="opt">
          <span>Width {strokeWidth}</span>
          <input type="range" min="1" max="24" bind:value={strokeWidth} />
        </label>
        {#if tool === 'text'}
          <label class="opt grow">
            <span>Text</span>
            <input type="text" bind:value={text} placeholder="Type text, then click the image…" />
          </label>
        {/if}
      </div>

      <div class="tool-group history-tools">
        <button class="ghost" onclick={undo} disabled={!hasUndo} title="Undo (Ctrl+Z)">↶</button>
        <button class="ghost" onclick={redo} disabled={!hasRedo} title="Redo (Ctrl+Shift+Z)">↷</button>
        <button class="ghost" onclick={revert} disabled={!dirty || loading} title="Revert to original">
          Reset
        </button>
      </div>
    </div>

    <div class="stage">
      <canvas
        bind:this={canvas}
        onpointerdown={onDown}
        onpointermove={onMove}
        onpointerup={onUp}
        onpointercancel={onUp}
      ></canvas>
      {#if loading}
        <div class="overlay">Loading image…</div>
      {/if}
    </div>
  </section>
{:else}
  <section class="view">
    <div class="empty">Nothing to edit.</div>
  </section>
{/if}

<style>
  .row {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
  }
  .head-actions {
    display: flex;
    gap: 0.5rem;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 0.9rem;
    padding: 0.7rem 0.8rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    margin-bottom: 0.9rem;
  }
  .tool-group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .options {
    gap: 0.75rem;
  }
  .history-tools {
    margin-left: auto;
  }
  .tool {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.1rem;
    padding: 0.35rem 0.5rem;
    min-width: 58px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-dim);
  }
  .tool:hover:not(:disabled) {
    background: var(--surface-3);
    border-color: var(--border);
    color: var(--text);
  }
  .tool.active {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border-color: var(--accent);
    color: var(--text);
  }
  .tool-icon {
    font-size: 1.05rem;
    line-height: 1.1;
  }
  .tool-label {
    font-size: 0.68rem;
  }
  .opt {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  .opt input[type='color'] {
    width: 42px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface-3);
    cursor: pointer;
  }
  .opt input[type='range'] {
    width: 110px;
  }
  .opt.grow {
    flex: 1;
    min-width: 180px;
  }
  .opt.grow input {
    width: 100%;
  }
  .stage {
    position: relative;
    display: grid;
    place-items: center;
    min-height: 320px;
    padding: 1rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 14px;
  }
  .stage canvas {
    max-width: 100%;
    max-height: calc(100vh - 300px);
    border-radius: 8px;
    background: var(--surface-3);
    box-shadow: 0 10px 32px rgba(0, 0, 0, 0.4);
    cursor: crosshair;
    touch-action: none;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text-dim);
    background: color-mix(in srgb, var(--surface-2) 70%, transparent);
    border-radius: 14px;
  }
  .empty {
    padding: 3rem;
    text-align: center;
    color: var(--text-dim);
    background: var(--surface-2);
    border: 1px dashed var(--border);
    border-radius: 14px;
  }
</style>
