<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api } from '../api';
  import { toast } from '../state.svelte';

  type Tool = 'rect' | 'ellipse' | 'arrow' | 'line' | 'pen' | 'text' | 'highlight' | 'pixelate' | 'blur' | 'step' | 'balloon' | 'magnify' | 'spotlight' | 'cursor' | 'cutout' | 'crop';

  const tools: { id: Tool; label: string; icon: string }[] = [
    { id: 'rect', label: 'Rectangle', icon: '▭' },
    { id: 'ellipse', label: 'Ellipse', icon: '◯' },
    { id: 'arrow', label: 'Arrow', icon: '➔' },
    { id: 'line', label: 'Line', icon: '╱' },
    { id: 'pen', label: 'Pen', icon: '✎' },
    { id: 'text', label: 'Text', icon: 'T' },
    { id: 'highlight', label: 'Highlight', icon: '▰' },
    { id: 'pixelate', label: 'Pixelate', icon: '▦' },
    { id: 'blur', label: 'Blur', icon: '◉' },
    { id: 'step', label: 'Step', icon: '①' },
    { id: 'balloon', label: 'Balloon', icon: '💬' },
    { id: 'magnify', label: 'Magnify', icon: '🔍' },
    { id: 'spotlight', label: 'Spotlight', icon: '🔦' },
    { id: 'cursor', label: 'Cursor', icon: '➤' },
    { id: 'cutout', label: 'Cut out', icon: '✂' },
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
  let filePath = $state('');
  let stepCounter = $state(1);
  let opacity = $state(100);
  let shapeFill = $state(false);
  let lineStyle = $state<'solid' | 'dashed' | 'dotted'>('solid');
  let arrowStart = $state(false);
  let arrowEnd = $state(true);
  let fontFamily = $state('system-ui');
  let magnifyZoom = $state(2);
  let balloonText = $state('');
  let cursorX = $state(0);
  let cursorY = $state(0);

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
    if (!canvas) return;
    loading = true;
    try {
      const info = await api.editorInfo();
      if (!info) {
        toast('Could not load image info', 'error');
        return;
      }
      filePath = info.path;
      const dataUrl = await api.readImageBase64(info.path);
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
      stepCounter = 1;
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
      case 'line':
      case 'pixelate':
      case 'blur':
      case 'highlight':
      case 'step':
      case 'balloon':
      case 'magnify':
      case 'spotlight':
      case 'cutout':
        drawPreview(context, p.x, p.y);
        commit();
        break;
      case 'cursor':
        placeCursor(context, p.x, p.y);
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

    // Advance the step counter only once the shape has actually been placed.
    if (tool === 'step') stepCounter++;
  }

  function drawPreview(context: CanvasRenderingContext2D, x: number, y: number) {
    const x0 = Math.min(startX, x);
    const y0 = Math.min(startY, y);
    const w = Math.abs(x - startX);
    const h = Math.abs(y - startY);

    context.lineWidth = strokeWidth;
    context.strokeStyle = color;
    context.fillStyle = color;

    context.globalAlpha = opacity / 100;
    // Reset any dash left over from a previous (line/pen) stroke.
    context.setLineDash([]);

    switch (tool) {
      case 'rect':
        context.beginPath();
        context.rect(x0, y0, w, h);
        if (shapeFill) {
          context.fillStyle = color;
          context.fill();
        }
        context.stroke();
        break;
      case 'ellipse':
        context.beginPath();
        context.ellipse(x0 + w / 2, y0 + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
        if (shapeFill) {
          context.fillStyle = color;
          context.fill();
        }
        context.stroke();
        break;
      case 'arrow':
        setLineStyle(context);
        drawArrow(context, startX, startY, x, y, arrowStart, arrowEnd);
        break;
      case 'line':
        setLineStyle(context);
        context.beginPath();
        context.moveTo(startX, startY);
        context.lineTo(x, y);
        context.stroke();
        break;
      case 'pen':
        context.setLineDash([]);
        break;
      case 'pixelate':
        applyPixelate(context, startX, startY, x, y);
        break;
      case 'blur':
        applyBlur(context, startX, startY, x, y);
        break;
      case 'highlight':
        context.fillStyle = color;
        context.fillRect(x0, y0, w, h);
        break;
      case 'step': {
        const cx = x0 + w / 2;
        const cy = y0 + h / 2;
        const r = Math.max(12, Math.min(w, h) / 2);
        context.beginPath();
        context.arc(cx, cy, r, 0, Math.PI * 2);
        context.fillStyle = color;
        context.fill();
        context.fillStyle = '#ffffff';
        context.font = `700 ${r}px ${fontFamily}, sans-serif`;
        context.textAlign = 'center';
        context.textBaseline = 'middle';
        context.fillText(String(stepCounter), cx, cy + 1);
        break;
      }
      case 'balloon':
        drawBalloon(context, x0, y0, w, h, balloonText, color);
        break;
      case 'magnify':
        drawMagnifier(context, x0, y0, w, h, magnifyZoom);
        break;
      case 'spotlight':
        drawSpotlight(context, x0, y0, w, h);
        break;
      case 'cutout':
        applyCutout(context, x0, y0, w, h);
        break;
      case 'crop':
        drawCropOverlay(context, x0, y0, w, h);
        break;
    }

    context.globalAlpha = 1;
  }

  function setLineStyle(context: CanvasRenderingContext2D) {
    context.lineWidth = strokeWidth;
    context.strokeStyle = color;
    if (lineStyle === 'dashed') context.setLineDash([strokeWidth * 3, strokeWidth * 2]);
    else if (lineStyle === 'dotted') context.setLineDash([strokeWidth * 0.5, strokeWidth * 2]);
    else context.setLineDash([]);
  }

  function drawArrow(
    context: CanvasRenderingContext2D,
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    start: boolean,
    end: boolean,
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

    if (end) {
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

    if (start) {
      const startAngle = Math.atan2(y0 - y1, x0 - x1);
      context.beginPath();
      context.moveTo(x0, y0);
      context.lineTo(
        x0 - head * Math.cos(startAngle - Math.PI / 6),
        y0 - head * Math.sin(startAngle - Math.PI / 6),
      );
      context.lineTo(
        x0 - head * Math.cos(startAngle + Math.PI / 6),
        y0 - head * Math.sin(startAngle + Math.PI / 6),
      );
      context.closePath();
      context.fill();
    }
  }

  function drawBalloon(
    context: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    text: string,
    bgColor: string,
  ) {
    const padding = 8;
    const fontSize = Math.max(12, strokeWidth * 3);
    context.font = `500 ${fontSize}px ${fontFamily}, sans-serif`;
    const metrics = context.measureText(text || 'Text');
    const textW = metrics.width;
    const textH = fontSize;
    const bw = textW + padding * 2;
    const bh = textH + padding * 2;
    const bx = x + w / 2 - bw / 2;
    const by = y + h / 2 - bh / 2;
    const tailH = 16;
    const tailW = 12;

    context.fillStyle = bgColor;
    context.strokeStyle = bgColor;
    context.lineWidth = 2;

    // Balloon body
    const r = 8;
    context.beginPath();
    context.moveTo(bx + r, by);
    context.lineTo(bx + bw - r, by);
    context.quadraticCurveTo(bx + bw, by, bx + bw, by + r);
    context.lineTo(bx + bw, by + bh - r);
    context.quadraticCurveTo(bx + bw, by + bh, bx + bw - r, by + bh);
    context.lineTo(bx + r, by + bh);
    context.quadraticCurveTo(bx, by + bh, bx, by + bh - r);
    context.lineTo(bx, by + r);
    context.quadraticCurveTo(bx, by, bx + r, by);
    context.closePath();
    context.fill();

    // Tail
    const tailX = bx + bw / 2;
    const tailY = by + bh;
    context.beginPath();
    context.moveTo(tailX - tailW / 2, tailY);
    context.lineTo(tailX + tailW / 2, tailY);
    context.lineTo(tailX, tailY + tailH);
    context.closePath();
    context.fill();

    // Text
    context.fillStyle = '#ffffff';
    context.textAlign = 'center';
    context.textBaseline = 'middle';
    context.fillText(text || 'Text', bx + bw / 2, by + bh / 2 + 1);
  }

  function drawMagnifier(
    context: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    zoom: number,
  ) {
    const c = canvas!;
    const cx = x + w / 2;
    const cy = y + h / 2;
    const r = Math.min(w, h) / 2;

    // Draw magnified content
    const srcR = r / zoom;
    context.save();
    context.beginPath();
    context.arc(cx, cy, r, 0, Math.PI * 2);
    context.clip();
    context.drawImage(c, cx - srcR, cy - srcR, srcR * 2, srcR * 2, cx - r, cy - r, r * 2, r * 2);
    context.restore();

    // Border
    context.beginPath();
    context.arc(cx, cy, r, 0, Math.PI * 2);
    context.strokeStyle = color;
    context.lineWidth = strokeWidth;
    context.stroke();

    // Handle
    const handleLen = r * 0.8;
    const handleAngle = Math.PI / 4;
    const hx = cx + r * Math.cos(handleAngle);
    const hy = cy + r * Math.sin(handleAngle);
    context.beginPath();
    context.moveTo(hx, hy);
    context.lineTo(hx + handleLen * Math.cos(handleAngle), hy + handleLen * Math.sin(handleAngle));
    context.stroke();
  }

  function drawSpotlight(
    context: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
  ) {
    const c = canvas!;
    context.save();
    context.fillStyle = 'rgba(0, 0, 0, 0.6)';
    context.beginPath();
    context.rect(0, 0, c.width, c.height);
    context.rect(x, y, w, h);
    context.fill('evenodd');
    context.restore();

    // Border around spotlight
    context.strokeStyle = color;
    context.lineWidth = Math.max(2, strokeWidth);
    context.strokeRect(x, y, w, h);
  }

  function applyCutout(
    context: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
  ) {
    context.save();
    context.clearRect(x, y, w, h);
    context.restore();
  }

  function placeCursor(context: CanvasRenderingContext2D, x: number, y: number) {
    const size = Math.max(16, strokeWidth * 4);
    context.fillStyle = '#ffffff';
    context.strokeStyle = '#000000';
    context.lineWidth = 2;

    context.beginPath();
    context.moveTo(x, y);
    context.lineTo(x, y + size);
    context.lineTo(x + size * 0.3, y + size * 0.75);
    context.lineTo(x + size * 0.5, y + size * 0.9);
    context.lineTo(x + size * 0.65, y + size * 0.7);
    context.lineTo(x + size * 0.9, y + size * 0.7);
    context.lineTo(x + size * 0.75, y + size * 0.5);
    context.lineTo(x + size, y + size * 0.3);
    context.closePath();
    context.fill();
    context.stroke();
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

    // Smaller block size for finer pixelation (ShareX-like).
    const block = Math.max(2, Math.round(strokeWidth * 1.2));
    const small = document.createElement('canvas');
    small.width = Math.max(1, Math.floor(w / block));
    small.height = Math.max(1, Math.floor(h / block));
    const sctx = small.getContext('2d')!;
    sctx.imageSmoothingEnabled = false;
    sctx.drawImage(canvas!, x, y, w, h, 0, 0, small.width, small.height);

    context.save();
    context.globalAlpha = 1;
    context.imageSmoothingEnabled = false;
    context.drawImage(small, 0, 0, small.width, small.height, x, y, w, h);
    context.restore();
  }

  function applyBlur(
    context: CanvasRenderingContext2D,
    x0: number,
    y0: number,
    x1: number,
    y1: number,
  ) {
    const { x, y, w, h } = region(x0, y0, x1, y1);
    if (w < 2 || h < 2) return;

    const radius = Math.max(2, strokeWidth);
    const temp = document.createElement('canvas');
    temp.width = w;
    temp.height = h;
    const tctx = temp.getContext('2d')!;
    tctx.filter = `blur(${radius}px)`;
    tctx.drawImage(canvas!, x, y, w, h, 0, 0, w, h);

    context.save();
    context.globalAlpha = 1;
    context.clearRect(x, y, w, h);
    context.drawImage(temp, x, y);
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
    if (!canvas || !filePath || saving) return;
    saving = true;
    try {
      const data = canvas.toDataURL('image/png');
      await api.applyEdits(filePath, data);
      toast('Saved', 'success');
      close();
    } catch (err) {
      toast(`Could not save: ${err}`, 'error');
    } finally {
      saving = false;
    }
  }

  function close() {
    const win = getCurrentWindow();
    win.close();
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

<section class="editor-window">
  <header class="view-head">
    <div class="title-group">
      <h1>Image Editor</h1>
      {#if !loading && dims.w > 0}
        <p class="sub">{dims.w} × {dims.h}px</p>
      {/if}
    </div>
    <div class="head-actions">
      <button class="ghost" onclick={undo} disabled={!hasUndo} title="Undo (Ctrl+Z)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>
        Undo
      </button>
      <button class="ghost" onclick={redo} disabled={!hasRedo} title="Redo (Ctrl+Shift+Z)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
        Redo
      </button>
      <button class="ghost" onclick={revert} disabled={!dirty || loading} title="Revert to original">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
        Reset
      </button>
      <span class="separator"></span>
      <button class="ghost" onclick={copy} disabled={loading || saving} title="Copy to clipboard">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        Copy
      </button>
      <button class="primary" onclick={save} disabled={loading || saving} title="Save and close">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>
        {saving ? 'Saving…' : 'Save & Close'}
      </button>
      <button class="ghost" onclick={close} title="Close editor">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    </div>
  </header>

  <div class="editor-body">
    <aside class="tool-panel">
      <div class="tool-section">
        <span class="section-label">Tools</span>
        <div class="tool-grid">
          {#each tools as item (item.id)}
            <button
              class="tool-btn"
              class:active={tool === item.id}
              title={item.label}
              onclick={() => (tool = item.id)}
            >
              <span class="tool-icon">{item.icon}</span>
            </button>
          {/each}
        </div>
      </div>
    </aside>

    <div class="canvas-area">
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
    </div>

    <aside class="options-panel">
      <div class="options-section">
        <span class="section-label">Color</span>
        <div class="color-row">
          <input type="color" bind:value={color} />
          <span class="color-value">{color}</span>
        </div>
        <div class="preset-colors">
          <button class="swatch" style="background:#ff3b30" onclick={() => color = '#ff3b30'} title="Red"></button>
          <button class="swatch" style="background:#ff9500" onclick={() => color = '#ff9500'} title="Orange"></button>
          <button class="swatch" style="background:#ffcc00" onclick={() => color = '#ffcc00'} title="Yellow"></button>
          <button class="swatch" style="background:#34c759" onclick={() => color = '#34c759'} title="Green"></button>
          <button class="swatch" style="background:#007aff" onclick={() => color = '#007aff'} title="Blue"></button>
          <button class="swatch" style="background:#af52de" onclick={() => color = '#af52de'} title="Purple"></button>
          <button class="swatch" style="background:#ffffff" onclick={() => color = '#ffffff'} title="White"></button>
          <button class="swatch" style="background:#000000" onclick={() => color = '#000000'} title="Black"></button>
        </div>
      </div>

      <div class="options-section">
        <span class="section-label">Size — {strokeWidth}px</span>
        <input type="range" min="1" max="24" bind:value={strokeWidth} />
        <div class="size-presets">
          <button class="size-preset" class:active={strokeWidth === 2} onclick={() => strokeWidth = 2}>S</button>
          <button class="size-preset" class:active={strokeWidth === 4} onclick={() => strokeWidth = 4}>M</button>
          <button class="size-preset" class:active={strokeWidth === 8} onclick={() => strokeWidth = 8}>L</button>
          <button class="size-preset" class:active={strokeWidth === 16} onclick={() => strokeWidth = 16}>XL</button>
        </div>
      </div>

      {#if tool === 'text'}
        <div class="options-section">
          <span class="section-label">Text</span>
          <input type="text" bind:value={text} placeholder="Enter text…" />
          <p class="hint">Click on the image to place text</p>
        </div>
      {/if}

      {#if tool === 'balloon'}
        <div class="options-section">
          <span class="section-label">Balloon Text</span>
          <input type="text" bind:value={balloonText} placeholder="Enter balloon text…" />
          <p class="hint">Draw a speech balloon on the image</p>
        </div>
      {/if}

      {#if tool === 'magnify'}
        <div class="options-section">
          <span class="section-label">Zoom — {magnifyZoom}x</span>
          <input type="range" min="1.5" max="5" step="0.5" bind:value={magnifyZoom} />
          <p class="hint">Magnification level</p>
        </div>
      {/if}

      {#if tool === 'blur' || tool === 'pixelate'}
        <div class="options-section">
          <span class="section-label">Intensity</span>
          <input type="range" min="1" max="24" bind:value={strokeWidth} />
          <p class="hint">Adjust the {tool === 'blur' ? 'blur radius' : 'pixel size'}</p>
        </div>
      {/if}

      {#if tool === 'rect' || tool === 'ellipse' || tool === 'highlight'}
        <div class="options-section">
          <span class="section-label">Fill</span>
          <label class="toggle-row">
            <input type="checkbox" bind:checked={shapeFill} />
            <span>Fill shape with color</span>
          </label>
        </div>
      {/if}

      {#if tool === 'line' || tool === 'arrow' || tool === 'pen'}
        <div class="options-section">
          <span class="section-label">Line Style</span>
          <div class="line-styles">
            <button class="line-btn" class:active={lineStyle === 'solid'} onclick={() => lineStyle = 'solid'} title="Solid">
              <svg width="24" height="8" viewBox="0 0 24 8"><line x1="1" y1="4" x2="23" y2="4" stroke="currentColor" stroke-width="2"/></svg>
            </button>
            <button class="line-btn" class:active={lineStyle === 'dashed'} onclick={() => lineStyle = 'dashed'} title="Dashed">
              <svg width="24" height="8" viewBox="0 0 24 8"><line x1="1" y1="4" x2="23" y2="4" stroke="currentColor" stroke-width="2" stroke-dasharray="4 3"/></svg>
            </button>
            <button class="line-btn" class:active={lineStyle === 'dotted'} onclick={() => lineStyle = 'dotted'} title="Dotted">
              <svg width="24" height="8" viewBox="0 0 24 8"><line x1="1" y1="4" x2="23" y2="4" stroke="currentColor" stroke-width="2" stroke-dasharray="0.5 3"/></svg>
            </button>
          </div>
        </div>
      {/if}

      {#if tool === 'arrow'}
        <div class="options-section">
          <span class="section-label">Arrowheads</span>
          <label class="toggle-row">
            <input type="checkbox" bind:checked={arrowStart} />
            <span>Start</span>
          </label>
          <label class="toggle-row">
            <input type="checkbox" bind:checked={arrowEnd} />
            <span>End</span>
          </label>
        </div>
      {/if}

      {#if tool === 'text' || tool === 'balloon' || tool === 'step'}
        <div class="options-section">
          <span class="section-label">Font</span>
          <select bind:value={fontFamily}>
            <option value="system-ui">System UI</option>
            <option value="serif">Serif</option>
            <option value="monospace">Monospace</option>
            <option value="cursive">Cursive</option>
          </select>
        </div>
      {/if}

      {#if tool !== 'crop' && tool !== 'cursor'}
        <div class="options-section">
          <span class="section-label">Opacity — {opacity}%</span>
          <input type="range" min="10" max="100" bind:value={opacity} />
        </div>
      {/if}

      <div class="options-section shortcuts">
        <span class="section-label">Shortcuts</span>
        <div class="shortcut-list">
          <div><kbd>Ctrl+Z</kbd> Undo</div>
          <div><kbd>Ctrl+Y</kbd> Redo</div>
          <div><kbd>Esc</kbd> Close</div>
        </div>
      </div>
    </aside>
  </div>
</section>

<style>
  .editor-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  /* ---- Header ---- */
  .view-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .title-group h1 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .sub {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .separator {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 0.3rem;
  }

  /* ---- Body: 3-column layout ---- */
  .editor-body {
    display: grid;
    grid-template-columns: 52px 1fr 240px;
    flex: 1;
    min-height: 0;
  }

  /* ---- Left tool panel ---- */
  .tool-panel {
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 0.5rem 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow-y: auto;
  }
  .tool-section {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .section-label {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    text-align: center;
    padding: 0.2rem 0;
  }
  .tool-grid {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .tool-btn {
    display: grid;
    place-items: center;
    width: 100%;
    aspect-ratio: 1;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.12s ease;
    padding: 0;
  }
  .tool-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tool-btn.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .tool-icon {
    font-size: 1.1rem;
    line-height: 1;
  }

  /* ---- Canvas area ---- */
  .canvas-area {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .stage {
    position: relative;
    display: grid;
    place-items: center;
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: #0d0f14;
    background-image:
      linear-gradient(45deg, #14161c 25%, transparent 25%),
      linear-gradient(-45deg, #14161c 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #14161c 75%),
      linear-gradient(-45deg, transparent 75%, #14161c 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0;
  }
  .stage canvas {
    max-width: 95%;
    max-height: 95%;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
    cursor: crosshair;
    touch-action: none;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text-dim);
    background: rgba(13, 15, 20, 0.8);
    font-size: 0.9rem;
  }

  /* ---- Right options panel ---- */
  .options-panel {
    background: var(--surface);
    border-left: 1px solid var(--border);
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    overflow-y: auto;
  }
  .options-section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .options-section .section-label {
    text-align: left;
    padding: 0;
  }
  .color-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .color-row input[type='color'] {
    width: 36px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-3);
    cursor: pointer;
  }
  .color-value {
    font-size: 0.72rem;
    color: var(--text-dim);
    font-family: ui-monospace, monospace;
  }
  .preset-colors {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }
  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    border: 1px solid var(--border);
    cursor: pointer;
    padding: 0;
    transition: transform 0.1s ease;
  }
  .swatch:hover {
    transform: scale(1.15);
  }
  .options-section input[type='range'] {
    width: 100%;
    accent-color: var(--accent);
  }
  .size-presets {
    display: flex;
    gap: 0.3rem;
  }
  .size-preset {
    flex: 1;
    padding: 0.3rem 0;
    font-size: 0.7rem;
    font-weight: 600;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.12s ease;
  }
  .size-preset:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .size-preset.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .options-section input[type='text'] {
    width: 100%;
    padding: 0.4rem 0.55rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-3);
    color: var(--text);
    font-size: 0.8rem;
  }
  .options-section input[type='text']:focus {
    outline: 2px solid color-mix(in srgb, var(--accent) 60%, transparent);
    outline-offset: 1px;
  }
  .hint {
    margin: 0;
    font-size: 0.68rem;
    color: var(--text-dim);
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--text);
    cursor: pointer;
  }
  .toggle-row input[type='checkbox'] {
    accent-color: var(--accent);
    width: 0.9rem;
    height: 0.9rem;
  }
  .line-styles {
    display: flex;
    gap: 0.3rem;
  }
  .line-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 24px;
    padding: 0;
    border-radius: 5px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-dim);
    cursor: pointer;
  }
  .line-btn:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .line-btn.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
    color: var(--accent);
  }
  .options-section select {
    width: 100%;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-3);
    color: var(--text);
    font-size: 0.78rem;
  }
  .shortcuts {
    margin-top: auto;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border);
  }
  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.7rem;
    color: var(--text-dim);
  }
  kbd {
    display: inline-block;
    padding: 0.1rem 0.3rem;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.65rem;
    font-family: ui-monospace, monospace;
    margin-right: 0.3rem;
  }

  /* ---- Buttons ---- */
  button {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.7rem;
    border-radius: 7px;
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    transition: all 0.12s ease;
    white-space: nowrap;
  }
  button:hover:not(:disabled) {
    background: var(--surface-3);
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-2);
    border-color: var(--accent-2);
  }
  button.ghost {
    background: transparent;
  }
  button.ghost:hover:not(:disabled) {
    background: var(--surface-2);
  }
</style>