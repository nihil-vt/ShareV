<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    Canvas,
    Rect,
    Ellipse,
    Circle,
    Triangle,
    Line,
    Path,
    Group,
    Textbox,
    FabricImage,
    PencilBrush,
    Shadow,
    ActiveSelection,
    filters,
    type FabricObject,
    type TMat2D,
    type TPointerEvent,
  } from 'fabric';
  import { api } from '../api';
  import { toast } from '../state.svelte';

  type Tool =
    | 'select'
    | 'rect'
    | 'ellipse'
    | 'arrow'
    | 'line'
    | 'pen'
    | 'text'
    | 'highlight'
    | 'pixelate'
    | 'blur'
    | 'step'
    | 'balloon'
    | 'magnify'
    | 'spotlight'
    | 'cursor'
    | 'cutout'
    | 'crop'
    | 'image'
    | 'emoji';

  const tools: { id: Tool; label: string; icon: string }[] = [
    { id: 'select', label: 'Select / move (V)', icon: '⤢' },
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
    { id: 'image', label: 'Insert image', icon: '🖼' },
    { id: 'emoji', label: 'Emoji stamp', icon: '😀' },
  ];

  const EMOJIS = ['😀', '😂', '😍', '🤔', '😎', '😭', '👍', '👎', '👏', '🙏', '❤️', '🔥', '⭐', '✅', '❌', '⚠️', '❓', '❗', '💡', '🎉', '👀', '💯', '🤝', '👋'];

  const MAX_UNDO = 30;
  const MIN_SIZE = 4;

  const toolHints: Record<Tool, string> = {
    select: 'Drag objects to move · corners to resize · double-click text to edit',
    rect: 'Drag to draw a rectangle',
    ellipse: 'Drag to draw an ellipse',
    arrow: 'Drag to draw an arrow',
    line: 'Drag to draw a line',
    pen: 'Draw freehand',
    text: 'Click to place text · double-click it later to edit',
    highlight: 'Drag to highlight',
    pixelate: 'Drag a region to pixelate · stays selectable',
    blur: 'Drag a region to blur · stays selectable',
    step: 'Click to place a step badge',
    balloon: 'Click to place a speech balloon',
    magnify: 'Drag a circle to magnify',
    spotlight: 'Drag to spotlight an area',
    cursor: 'Click to stamp a cursor',
    cutout: 'Drag a region to cut out',
    crop: 'Drag a region to crop to',
    image: 'Click the canvas to choose an image to stamp',
    emoji: 'Click to stamp the emoji',
  };

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let stageEl = $state<HTMLDivElement | null>(null);
  let fcanvas: Canvas | null = null;
  let tool = $state<Tool>('select');
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
  let fontSize = $state(28);
  let fontBold = $state(true);
  let fontItalic = $state(false);
  let textAlign = $state<'left' | 'center' | 'right'>('left');
  let cornerRadius = $state(0);
  let shapeShadow = $state(false);
  let emojiChoice = $state('😀');
  let magnifyZoom = $state(2);
  let balloonText = $state('');
  let zoom = $state(1);
  let zoomLabel = $state('Fit');

  // Undo/redo hold full canvas JSON snapshots (objects + background).
  let undoStack: string[] = [];
  let redoStack: string[] = [];
  let suppressHistory = false;
  let originalDataUrl: string | null = null;

  // Transient drag state for shape tools.
  let dragObj: FabricObject | null = null;
  let dragStart: { x: number; y: number } | null = null;

  function sync() {
    hasUndo = undoStack.length > 0;
    hasRedo = redoStack.length > 0;
  }

  function pushHistory() {
    if (suppressHistory || !fcanvas) return;
    undoStack.push(JSON.stringify(fcanvas.toJSON()));
    if (undoStack.length > MAX_UNDO) undoStack.shift();
    redoStack = [];
    dirty = true;
    sync();
  }

  async function restore(json: string) {
    if (!fcanvas) return;
    suppressHistory = true;
    try {
      await fcanvas.loadFromJSON(JSON.parse(json));
    } finally {
      suppressHistory = false;
    }
    fcanvas.requestRenderAll();
    syncDims();
    sync();
  }

  async function undo() {
    if (!fcanvas || undoStack.length === 0) return;
    redoStack.push(JSON.stringify(fcanvas.toJSON()));
    const prev = undoStack.pop()!;
    dirty = true;
    await restore(prev);
  }

  async function redo() {
    if (!fcanvas || redoStack.length === 0) return;
    undoStack.push(JSON.stringify(fcanvas.toJSON()));
    const next = redoStack.pop()!;
    dirty = true;
    await restore(next);
  }

  function syncDims() {
    if (fcanvas) dims = { w: fcanvas.getWidth(), h: fcanvas.getHeight() };
  }

  // -------------------------------------------------------------------------
  // Setup / loading
  // -------------------------------------------------------------------------

  async function load() {
    if (!canvasEl) return;
    loading = true;
    try {
      const info = await api.editorInfo();
      if (!info) {
        toast('Could not load image info', 'error');
        return;
      }
      filePath = info.path;
      const dataUrl = await api.readImageBase64(info.path);
      originalDataUrl = dataUrl;

      fcanvas = new Canvas(canvasEl, {
        enableRetinaScaling: false,
        preserveObjectStacking: true,
        stopContextMenu: true,
      });

      const bg = await FabricImage.fromURL(dataUrl);
      bg.set({ selectable: false, evented: false, excludeFromExport: false });
      const w = bg.width || info.width;
      const h = bg.height || info.height;
      fcanvas.backgroundImage = bg;
      fcanvas.setDimensions({ width: w, height: h });
      syncDims();

      fcanvas.on('object:added', () => pushHistory());
      fcanvas.on('object:modified', () => pushHistory());
      fcanvas.on('object:removed', () => pushHistory());
      fcanvas.on('path:created', () => pushHistory());
      fcanvas.on('mouse:down', onCanvasDown);
      fcanvas.on('mouse:move', onCanvasMove);
      fcanvas.on('mouse:up', onCanvasUp);

      suppressHistory = true;
      undoStack = [];
      redoStack = [];
      suppressHistory = false;
      dirty = false;
      stepCounter = 1;

      applyToolMode();
      fitZoom();
      fcanvas.requestRenderAll();
      sync();
    } catch (err) {
      toast(`Could not open the editor: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }

  function bgElement(): HTMLImageElement | HTMLCanvasElement | null {
    const bg = fcanvas?.backgroundImage;
    if (bg instanceof FabricImage) {
      const el = bg.getElement() as unknown;
      if (el instanceof HTMLImageElement || el instanceof HTMLCanvasElement) return el;
    }
    return null;
  }

  // -------------------------------------------------------------------------
  // Zoom
  // -------------------------------------------------------------------------

  function applyZoom(z: number, label: string) {
    if (!fcanvas) return;
    zoom = z;
    zoomLabel = label;
    fcanvas.setZoom(z);
    fcanvas.setDimensions(
      { width: dims.w * z, height: dims.h * z },
      { cssOnly: true },
    );
    fcanvas.requestRenderAll();
  }

  function fitZoom() {
    if (!fcanvas || !stageEl || dims.w === 0) return;
    const rect = stageEl.getBoundingClientRect();
    const availW = Math.max(200, rect.width - 48);
    const availH = Math.max(200, rect.height - 48);
    const z = Math.min(1, availW / dims.w, availH / dims.h);
    applyZoom(z, z >= 1 ? '100%' : `${Math.round(z * 100)}%`);
  }

  function zoomIn() {
    const z = Math.min(4, +(zoom + (zoom < 1 ? 0.25 : 0.5)).toFixed(2));
    applyZoom(z, `${Math.round(z * 100)}%`);
  }

  function zoomOut() {
    const z = Math.max(0.1, +(zoom - (zoom <= 1 ? 0.25 : 0.5)).toFixed(2));
    applyZoom(z, `${Math.round(z * 100)}%`);
  }

  function zoomActual() {
    applyZoom(1, '100%');
  }

  // -------------------------------------------------------------------------
  // Tool mode (selection vs free drawing)
  // -------------------------------------------------------------------------

  function applyToolMode() {
    if (!fcanvas) return;
    if (tool === 'pen') {
      fcanvas.isDrawingMode = true;
      const brush = new PencilBrush(fcanvas);
      brush.color = color;
      brush.width = strokeWidth;
      fcanvas.freeDrawingBrush = brush;
      fcanvas.selection = false;
      fcanvas.discardActiveObject();
    } else {
      fcanvas.isDrawingMode = false;
      fcanvas.selection = tool === 'select';
      if (tool !== 'select') fcanvas.discardActiveObject();
    }
    fcanvas.requestRenderAll();
  }

  function selectTool(id: Tool) {
    tool = id;
    applyToolMode();
  }

  function dashFor(): number[] | null {
    if (lineStyle === 'dashed') return [strokeWidth * 3, strokeWidth * 2];
    if (lineStyle === 'dotted') return [Math.max(1, strokeWidth * 0.5), strokeWidth * 2];
    return null;
  }

  // -------------------------------------------------------------------------
  // Shape factories (scene coordinates)
  // -------------------------------------------------------------------------

  function shadowFor(): { shadow?: Shadow } {
    if (!shapeShadow) return {};
    return {
      shadow: new Shadow({
        color: 'rgba(0, 0, 0, 0.45)',
        blur: Math.max(2, strokeWidth * 2),
        offsetX: Math.max(2, strokeWidth),
        offsetY: Math.max(2, strokeWidth),
      }),
    };
  }

  function makeRect(x: number, y: number, w: number, h: number): Rect {
    return new Rect({
      left: x,
      top: y,
      width: Math.max(1, w),
      height: Math.max(1, h),
      rx: cornerRadius,
      ry: cornerRadius,
      fill: shapeFill ? color : 'transparent',
      stroke: color,
      strokeWidth,
      strokeUniform: true,
      opacity: opacity / 100,
      ...shadowFor(),
    });
  }

  function makeEllipse(x: number, y: number, w: number, h: number): Ellipse {
    return new Ellipse({
      left: x,
      top: y,
      rx: Math.max(1, w / 2),
      ry: Math.max(1, h / 2),
      fill: shapeFill ? color : 'transparent',
      stroke: color,
      strokeWidth,
      strokeUniform: true,
      opacity: opacity / 100,
      ...shadowFor(),
    });
  }

  function makeLine(x0: number, y0: number, x1: number, y1: number): Line {
    return new Line([x0, y0, x1, y1], {
      stroke: color,
      strokeWidth,
      strokeUniform: true,
      strokeDashArray: dashFor() ?? undefined,
      strokeLineCap: 'round',
      opacity: opacity / 100,
      ...shadowFor(),
    });
  }

  function arrowPathString(
    x0: number,
    y0: number,
    x1: number,
    y1: number,
    withStart: boolean,
    withEnd: boolean,
  ): string {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;
    const px = -uy;
    const py = ux;
    const hw = strokeWidth / 2;
    const headLen = Math.max(10, strokeWidth * 3.5);
    const headHalf = headLen * 0.45;

    const tailX0 = withStart ? x0 + ux * headLen : x0;
    const tailY0 = withStart ? y0 + uy * headLen : y0;
    const tailX1 = withEnd ? x1 - ux * headLen : x1;
    const tailY1 = withEnd ? y1 - uy * headLen : y1;

    const pts: Array<[number, number]> = [];
    if (withStart) {
      pts.push([x0, y0]);
      pts.push([x0 + ux * headLen + px * headHalf, y0 + uy * headLen + py * headHalf]);
      pts.push([tailX0 + px * hw, tailY0 + py * hw]);
    } else {
      pts.push([tailX0 + px * hw, tailY0 + py * hw]);
    }
    if (withEnd) {
      pts.push([tailX1 + px * hw, tailY1 + py * hw]);
      pts.push([x1 - ux * headLen + px * headHalf, y1 - uy * headLen + py * headHalf]);
      pts.push([x1, y1]);
      pts.push([x1 - ux * headLen - px * headHalf, y1 - uy * headLen - py * headHalf]);
      pts.push([tailX1 - px * hw, tailY1 - py * hw]);
    } else {
      pts.push([tailX1 - px * hw, tailY1 - py * hw]);
    }
    if (withStart) {
      pts.push([tailX0 - px * hw, tailY0 - py * hw]);
      pts.push([x0 + ux * headLen - px * headHalf, y0 + uy * headLen - py * headHalf]);
    } else {
      pts.push([tailX0 - px * hw, tailY0 - py * hw]);
    }
    const f = (n: number) => +n.toFixed(2);
    return `M ${f(pts[0][0])} ${f(pts[0][1])} ` + pts.slice(1).map(([a, b]) => `L ${f(a)} ${f(b)}`).join(' ') + ' Z';
  }

  function makeArrow(x0: number, y0: number, x1: number, y1: number): Path {
    return new Path(arrowPathString(x0, y0, x1, y1, arrowStart, arrowEnd), {
      fill: color,
      stroke: color,
      strokeWidth: 1,
      opacity: opacity / 100,
      ...shadowFor(),
    });
  }

  function makeHighlight(x: number, y: number, w: number, h: number): Rect {
    return new Rect({
      left: x,
      top: y,
      width: Math.max(1, w),
      height: Math.max(1, h),
      fill: color,
      stroke: 'transparent',
      opacity: (opacity / 100) * 0.45,
    });
  }

  function makeTextAt(x: number, y: number): Textbox | null {
    const value = text.trim() || 'Text';
    const tb = new Textbox(value, {
      left: x,
      top: y,
      fontFamily,
      fontSize,
      fill: color,
      fontWeight: fontBold ? 700 : 400,
      fontStyle: fontItalic ? 'italic' : 'normal',
      textAlign,
      opacity: opacity / 100,
      editable: true,
      width: Math.max(60, value.length * fontSize * 0.65),
      ...shadowFor(),
    });
    return tb;
  }

  function makeEmojiAt(x: number, y: number): Textbox {
    return new Textbox(emojiChoice, {
      left: x,
      top: y,
      fontSize: Math.max(24, strokeWidth * 8),
      editable: false,
      opacity: opacity / 100,
      width: Math.max(48, strokeWidth * 10),
    });
  }

  async function placeImageAt(x: number, y: number) {
    if (!fcanvas) return;
    try {
      const picked = await api.pickFile();
      if (!picked) return;
      const dataUrl = await api.readImageBase64(picked);
      const img = await FabricImage.fromURL(dataUrl);
      const maxW = Math.max(48, dims.w / 3 || 320);
      const scale = Math.min(1, maxW / Math.max(1, img.width || 1));
      img.set({ left: x, top: y, scaleX: scale, scaleY: scale });
      fcanvas.add(img);
      fcanvas.setActiveObject(img);
      fcanvas.requestRenderAll();
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  function makeStepAt(x: number, y: number): Group {
    const r = Math.max(14, strokeWidth * 4);
    const circle = new Circle({
      radius: r,
      fill: color,
      stroke: color,
      originX: 'center',
      originY: 'center',
    });
    const label = new Textbox(String(stepCounter), {
      fontFamily,
      fontSize: r * 1.05,
      fontWeight: 700,
      fill: '#ffffff',
      textAlign: 'center',
      originX: 'center',
      originY: 'center',
      editable: false,
      width: r * 2,
    });
    return new Group([circle, label], { left: x, top: y, opacity: opacity / 100 });
  }

  function makeBalloonAt(x: number, y: number): Group {
    const value = balloonText.trim() || 'Text';
    const fontPx = Math.max(14, strokeWidth * 3.5);
    const pad = 10;
    const approxW = value.length * fontPx * 0.6 + pad * 2;
    const approxH = fontPx + pad * 2;
    const body = new Rect({
      width: approxW,
      height: approxH,
      rx: 9,
      ry: 9,
      fill: color,
      stroke: color,
      originX: 'center',
      originY: 'center',
    });
    const tail = new Triangle({
      width: 18,
      height: 20,
      fill: color,
      originX: 'center',
      originY: 'top',
      top: approxH / 2 - 2,
    });
    const label = new Textbox(value, {
      fontFamily,
      fontSize: fontPx,
      fill: '#ffffff',
      textAlign: 'center',
      originX: 'center',
      originY: 'center',
      editable: true,
      width: approxW - pad,
    });
    return new Group([body, tail, label], { left: x, top: y, opacity: opacity / 100 });
  }

  function makeCursorAt(x: number, y: number): Path {
    const s = Math.max(16, strokeWidth * 4) / 24;
    const d =
      'M 0 0 L 0 20 L 5.5 15.5 L 8 20.5 L 10.5 19.5 L 8 14.5 L 12.5 14.5 Z';
    const p = new Path(d, {
      left: x,
      top: y,
      fill: '#ffffff',
      stroke: '#000000',
      strokeWidth: 1.5 / s,
      scaleX: s,
      scaleY: s,
      strokeUniform: true,
    });
    return p;
  }

  // -------------------------------------------------------------------------
  // Region effects (pixelate / blur / magnify / spotlight / cutout / crop)
  // -------------------------------------------------------------------------

  function cropSource(x: number, y: number, w: number, h: number): HTMLCanvasElement | null {
    const el = bgElement();
    if (!el || !fcanvas) return null;
    const W = fcanvas.getWidth();
    const H = fcanvas.getHeight();
    const cx = Math.max(0, Math.min(x, W));
    const cy = Math.max(0, Math.min(y, H));
    const cw = Math.max(1, Math.min(w, W - cx));
    const ch = Math.max(1, Math.min(h, H - cy));
    const tmp = document.createElement('canvas');
    tmp.width = Math.floor(cw);
    tmp.height = Math.floor(ch);
    const tctx = tmp.getContext('2d')!;
    const sw = el instanceof HTMLImageElement ? el.naturalWidth || el.width : el.width;
    const sh = el instanceof HTMLImageElement ? el.naturalHeight || el.height : el.height;
    tctx.drawImage(el, (cx / W) * sw, (cy / H) * sh, (cw / W) * sw, (ch / H) * sh, 0, 0, tmp.width, tmp.height);
    return tmp;
  }

  async function addImageObject(
    source: HTMLCanvasElement,
    left: number,
    top: number,
    extra?: (img: FabricImage) => void,
  ) {
    if (!fcanvas) return;
    const img = await FabricImage.fromURL(source.toDataURL());
    img.set({ left, top, selectable: true, evented: true });
    extra?.(img);
    fcanvas.add(img);
    fcanvas.setActiveObject(img);
    fcanvas.requestRenderAll();
  }

  async function applyPixelate(x0: number, y0: number, x1: number, y1: number) {
    const x = Math.min(x0, x1);
    const y = Math.min(y0, y1);
    const w = Math.abs(x1 - x0);
    const h = Math.abs(y1 - y0);
    if (w < MIN_SIZE || h < MIN_SIZE) return;
    const src = cropSource(x, y, w, h);
    if (!src) return;
    const block = Math.max(2, Math.round(strokeWidth * 1.2));
    await addImageObject(src, x, y, (img) => {
      img.filters = [new filters.Pixelate({ blocksize: block })];
      img.applyFilters();
    });
  }

  async function applyBlur(x0: number, y0: number, x1: number, y1: number) {
    const x = Math.min(x0, x1);
    const y = Math.min(y0, y1);
    const w = Math.abs(x1 - x0);
    const h = Math.abs(y1 - y0);
    if (w < MIN_SIZE || h < MIN_SIZE) return;
    const src = cropSource(x, y, w, h);
    if (!src) return;
    const radius = Math.max(1, Math.round(strokeWidth));
    await addImageObject(src, x, y, (img) => {
      img.filters = [new filters.Blur({ blur: radius / 20 })];
      img.applyFilters();
    });
  }

  async function applyMagnify(x0: number, y0: number, x1: number, y1: number) {
    if (!fcanvas) return;
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    const r = Math.max(24, Math.min(Math.abs(x1 - x0), Math.abs(y1 - y0)) / 2);
    const srcR = r / magnifyZoom;
    const src = cropSource(cx - srcR, cy - srcR, srcR * 2, srcR * 2);
    if (!src) return;
    // Bake the circular clip into the bitmap so no fabric clipPath math is needed.
    const out = document.createElement('canvas');
    out.width = out.height = Math.ceil(srcR * 2);
    const octx = out.getContext('2d')!;
    octx.beginPath();
    octx.arc(srcR, srcR, srcR, 0, Math.PI * 2);
    octx.clip();
    octx.drawImage(src, 0, 0);
    const scale = r / srcR;
    await addImageObject(out, cx - r, cy - r, (img) => {
      img.set({ scaleX: scale, scaleY: scale });
    });
    const ring = new Circle({
      left: cx - r,
      top: cy - r,
      radius: r,
      fill: 'transparent',
      stroke: color,
      strokeWidth,
      strokeUniform: true,
      selectable: true,
      evented: true,
    });
    const handleLen = r * 0.7;
    const ha = Math.PI / 4;
    const handle = new Line(
      [cx + r * Math.cos(ha), cy + r * Math.sin(ha), cx + (r + handleLen) * Math.cos(ha), cy + (r + handleLen) * Math.sin(ha)],
      { stroke: color, strokeWidth: Math.max(3, strokeWidth), strokeUniform: true, strokeLineCap: 'round' },
    );
    fcanvas.add(ring, handle);
    fcanvas.requestRenderAll();
  }

  function applySpotlight(x0: number, y0: number, x1: number, y1: number) {
    if (!fcanvas) return;
    const W = fcanvas.getWidth();
    const H = fcanvas.getHeight();
    const x = Math.min(x0, x1);
    const y = Math.min(y0, y1);
    const w = Math.abs(x1 - x0);
    const h = Math.abs(y1 - y0);
    if (w < MIN_SIZE || h < MIN_SIZE) return;
    const shade = 'rgba(0, 0, 0, 0.6)';
    const parts = [
      new Rect({ left: 0, top: 0, width: W, height: y, fill: shade }),
      new Rect({ left: 0, top: y + h, width: W, height: H - y - h, fill: shade }),
      new Rect({ left: 0, top: y, width: x, height: h, fill: shade }),
      new Rect({ left: x + w, top: y, width: W - x - w, height: h, fill: shade }),
      new Rect({
        left: x,
        top: y,
        width: w,
        height: h,
        fill: 'transparent',
        stroke: color,
        strokeWidth: Math.max(2, strokeWidth),
        strokeUniform: true,
      }),
    ];
    const group = new Group(parts, { selectable: true, evented: true });
    fcanvas.add(group);
    fcanvas.setActiveObject(group);
    fcanvas.requestRenderAll();
  }

  async function applyCutout(x0: number, y0: number, x1: number, y1: number) {
    if (!fcanvas) return;
    const W = fcanvas.getWidth();
    const H = fcanvas.getHeight();
    const el = bgElement();
    if (!el) return;
    const x = Math.max(0, Math.min(x0, x1));
    const y = Math.max(0, Math.min(y0, y1));
    const w = Math.min(W - x, Math.abs(x1 - x0));
    const h = Math.min(H - y, Math.abs(y1 - y0));
    if (w < MIN_SIZE || h < MIN_SIZE) return;
    const tmp = document.createElement('canvas');
    tmp.width = W;
    tmp.height = H;
    const tctx = tmp.getContext('2d')!;
    tctx.drawImage(el, 0, 0, W, H);
    tctx.clearRect(x, y, w, h);
    const img = await FabricImage.fromURL(tmp.toDataURL());
    img.set({ selectable: false, evented: false });
    suppressHistory = true;
    fcanvas.backgroundImage = img;
    suppressHistory = false;
    fcanvas.requestRenderAll();
    pushHistory();
  }

  async function applyCropSel(x0: number, y0: number, x1: number, y1: number) {
    if (!fcanvas) return;
    const W = fcanvas.getWidth();
    const H = fcanvas.getHeight();
    const x = Math.max(0, Math.floor(Math.min(x0, x1)));
    const y = Math.max(0, Math.floor(Math.min(y0, y1)));
    const w = Math.min(W - x, Math.floor(Math.abs(x1 - x0)));
    const h = Math.min(H - y, Math.floor(Math.abs(y1 - y0)));
    if (w < MIN_SIZE || h < MIN_SIZE) return;
    const src = cropSource(x, y, w, h);
    if (!src) return;
    suppressHistory = true;
    try {
      const img = await FabricImage.fromURL(src.toDataURL());
      img.set({ selectable: false, evented: false });
      fcanvas.backgroundImage = img;
      fcanvas.setDimensions({ width: w, height: h });
      for (const obj of fcanvas.getObjects()) {
        obj.set({ left: (obj.left ?? 0) - x, top: (obj.top ?? 0) - y });
        obj.setCoords();
      }
      syncDims();
      fitZoom();
    } finally {
      suppressHistory = false;
    }
    fcanvas.requestRenderAll();
    pushHistory();
  }

  // -------------------------------------------------------------------------
  // Mouse interaction
  // -------------------------------------------------------------------------

  function onCanvasDown(opt: { e: TPointerEvent }) {
    if (!fcanvas || loading || saving) return;
    if (tool === 'select' || tool === 'pen') return;
    const p = fcanvas.getScenePoint(opt.e);
    dragStart = { x: p.x, y: p.y };

    if (tool === 'text') {
      const tb = makeTextAt(p.x, p.y);
      if (tb) {
        fcanvas.add(tb);
        fcanvas.setActiveObject(tb);
        tb.enterEditing();
        tb.selectAll();
      }
      dragStart = null;
      return;
    }
    if (tool === 'step') {
      const g = makeStepAt(p.x, p.y);
      fcanvas.add(g);
      fcanvas.setActiveObject(g);
      stepCounter++;
      dragStart = null;
      return;
    }
    if (tool === 'balloon') {
      const g = makeBalloonAt(p.x, p.y);
      fcanvas.add(g);
      fcanvas.setActiveObject(g);
      dragStart = null;
      return;
    }
    if (tool === 'cursor') {
      fcanvas.add(makeCursorAt(p.x, p.y));
      dragStart = null;
      return;
    }
    if (tool === 'emoji') {
      const stamp = makeEmojiAt(p.x, p.y);
      fcanvas.add(stamp);
      fcanvas.setActiveObject(stamp);
      dragStart = null;
      return;
    }
    if (tool === 'image') {
      dragStart = null;
      void placeImageAt(p.x, p.y);
      return;
    }

    // Drag tools: create a live object and resize it on move.
    let obj: FabricObject | null = null;
    if (tool === 'rect') obj = makeRect(p.x, p.y, 1, 1);
    else if (tool === 'ellipse') obj = makeEllipse(p.x, p.y, 1, 1);
    else if (tool === 'highlight') obj = makeHighlight(p.x, p.y, 1, 1);
    else if (tool === 'line') obj = makeLine(p.x, p.y, p.x + 1, p.y + 1);
    else if (tool === 'arrow') obj = makeArrow(p.x, p.y, p.x + 1, p.y + 1);
    if (obj) {
      suppressHistory = true;
      dragObj = obj;
      fcanvas.add(obj);
      suppressHistory = false;
    }
  }

  function onCanvasMove(opt: { e: TPointerEvent }) {
    if (!fcanvas || !dragObj || !dragStart) return;
    const p = fcanvas.getScenePoint(opt.e);
    const x = Math.min(dragStart.x, p.x);
    const y = Math.min(dragStart.y, p.y);
    const w = Math.abs(p.x - dragStart.x);
    const h = Math.abs(p.y - dragStart.y);
    if (dragObj instanceof Rect) {
      dragObj.set({ left: x, top: y, width: Math.max(1, w), height: Math.max(1, h) });
    } else if (dragObj instanceof Ellipse) {
      dragObj.set({ left: x, top: y, rx: Math.max(1, w / 2), ry: Math.max(1, h / 2) });
    } else if (dragObj instanceof Line) {
      dragObj.set({ x1: dragStart.x, y1: dragStart.y, x2: p.x, y2: p.y });
    } else if (dragObj instanceof Path) {
      const next = makeArrow(dragStart.x, dragStart.y, p.x, p.y);
      const idx = fcanvas.getObjects().indexOf(dragObj);
      suppressHistory = true;
      fcanvas.remove(dragObj);
      suppressHistory = false;
      dragObj = next;
      suppressHistory = true;
      if (idx >= 0) fcanvas.insertAt(idx, next);
      else fcanvas.add(next);
      suppressHistory = false;
    }
    dragObj.setCoords();
    fcanvas.requestRenderAll();
  }

  async function onCanvasUp(opt: { e: TPointerEvent }) {
    if (!fcanvas) return;
    if (tool === 'select' || tool === 'pen') return;
    const p = fcanvas.getScenePoint(opt.e);

    if (dragObj && dragStart) {
      const w = Math.abs(p.x - dragStart.x);
      const h = Math.abs(p.y - dragStart.y);
      const done = dragObj;
      dragObj = null;
      dragStart = null;
      if (w < MIN_SIZE && h < MIN_SIZE && (done instanceof Rect || done instanceof Ellipse)) {
        suppressHistory = true;
        fcanvas.remove(done);
        suppressHistory = false;
        fcanvas.requestRenderAll();
        return;
      }
      fcanvas.setActiveObject(done);
      pushHistory();
      return;
    }

    if (!dragStart) return;
    const sx = dragStart.x;
    const sy = dragStart.y;
    dragStart = null;
    switch (tool) {
      case 'pixelate':
        await applyPixelate(sx, sy, p.x, p.y);
        break;
      case 'blur':
        await applyBlur(sx, sy, p.x, p.y);
        break;
      case 'magnify':
        await applyMagnify(sx, sy, p.x, p.y);
        break;
      case 'spotlight':
        applySpotlight(sx, sy, p.x, p.y);
        break;
      case 'cutout':
        await applyCutout(sx, sy, p.x, p.y);
        break;
      case 'crop':
        await applyCropSel(sx, sy, p.x, p.y);
        break;
    }
  }

  function deleteSelection() {
    if (!fcanvas) return;
    const active = fcanvas.getActiveObjects();
    if (active.length === 0) return;
    fcanvas.discardActiveObject();
    fcanvas.remove(...active);
    fcanvas.requestRenderAll();
  }

  async function duplicateSelection() {
    if (!fcanvas) return;
    const active = fcanvas.getActiveObjects();
    if (active.length === 0) return;
    const clones = await Promise.all(active.map((o) => o.clone()));
    for (const clone of clones) {
      clone.set({ left: (clone.left ?? 0) + 12, top: (clone.top ?? 0) + 12 });
      clone.setCoords();
      fcanvas.add(clone);
    }
    fcanvas.setActiveObject(
      clones.length === 1 ? clones[0] : new ActiveSelection(clones, { canvas: fcanvas }),
    );
    fcanvas.requestRenderAll();
  }

  // -------------------------------------------------------------------------
  // Export / save / revert
  // -------------------------------------------------------------------------

  function exportDataUrl(): string | null {
    if (!fcanvas) return null;
    const prevZoom = zoom;
    const prevLabel = zoomLabel;
    const prevT = fcanvas.viewportTransform ? ([...fcanvas.viewportTransform] as TMat2D) : undefined;
    fcanvas.setZoom(1);
    fcanvas.setViewportTransform([1, 0, 0, 1, 0, 0]);
    fcanvas.setDimensions({ width: dims.w, height: dims.h }, { cssOnly: true });
    fcanvas.discardActiveObject();
    fcanvas.requestRenderAll();
    const data = fcanvas.toDataURL({ format: 'png', multiplier: 1 });
    if (prevT) fcanvas.setViewportTransform(prevT);
    applyZoom(prevZoom, prevLabel);
    return data;
  }

  async function copy() {
    const data = exportDataUrl();
    if (!data) return;
    try {
      await api.copyEdits(data);
      toast('Copied annotated image to clipboard', 'success');
    } catch (err) {
      toast(String(err), 'error');
    }
  }

  async function save() {
    if (!fcanvas || !filePath || saving) return;
    saving = true;
    try {
      const data = exportDataUrl();
      if (!data) throw new Error('could not render the image');
      await api.applyEdits(filePath, data);
      toast('Saved', 'success');
      close();
    } catch (err) {
      toast(`Could not save: ${err}`, 'error');
    } finally {
      saving = false;
    }
  }

  async function revert() {
    if (!fcanvas || !originalDataUrl) return;
    suppressHistory = true;
    try {
      const bg = await FabricImage.fromURL(originalDataUrl);
      bg.set({ selectable: false, evented: false });
      for (const obj of [...fcanvas.getObjects()]) fcanvas.remove(obj);
      fcanvas.backgroundImage = bg;
      const w = bg.width || dims.w;
      const h = bg.height || dims.h;
      fcanvas.setDimensions({ width: w, height: h });
      syncDims();
      fitZoom();
    } finally {
      suppressHistory = false;
    }
    dirty = true;
    pushHistory();
    fcanvas.requestRenderAll();
  }

  function close() {
    getCurrentWindow().close();
  }

  function editingText(): boolean {
    const active = fcanvas?.getActiveObject();
    return !!active && active instanceof Textbox && active.isEditing;
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const typing =
      target &&
      (target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable);
    const mod = e.ctrlKey || e.metaKey;
    const inTextEdit = editingText();

    if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) {
      e.preventDefault();
      void undo();
    } else if (mod && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) {
      e.preventDefault();
      void redo();
    } else if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void save();
    } else if (mod && e.key.toLowerCase() === 'c' && !typing) {
      e.preventDefault();
      void copy();
    } else if (mod && e.key.toLowerCase() === 'd' && !typing) {
      e.preventDefault();
      void duplicateSelection();
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && !typing && !inTextEdit) {
      e.preventDefault();
      deleteSelection();
    } else if (e.key.toLowerCase() === 'v' && !typing && !mod && !inTextEdit) {
      selectTool('select');
    } else if (e.key === 'Enter' && !typing && !mod && !inTextEdit) {
      e.preventDefault();
      void save();
    } else if (e.key === 'Escape') {
      if (inTextEdit) {
        (fcanvas?.getActiveObject() as Textbox | null | undefined)?.exitEditing?.();
      } else if (fcanvas?.getActiveObject() && tool === 'select') {
        fcanvas.discardActiveObject();
      } else if (!typing) close();
    }
  }

  onMount(() => {
    void load();
    window.addEventListener('keydown', onKey);
    window.addEventListener('resize', fitZoom);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('resize', fitZoom);
      fcanvas?.dispose();
      fcanvas = null;
    };
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
      <button class="ghost" onclick={() => selectTool('select')} class:active-tool={tool === 'select'} title="Select (V)">Select</button>
      <button class="ghost" onclick={() => void duplicateSelection()} title="Duplicate selection (Ctrl+D)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        Duplicate
      </button>
      <button class="ghost" onclick={() => deleteSelection()} title="Delete selection (Del)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
        Delete
      </button>
      <button class="ghost" onclick={() => void undo()} disabled={!hasUndo} title="Undo (Ctrl+Z)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>
        Undo
      </button>
      <button class="ghost" onclick={() => void redo()} disabled={!hasRedo} title="Redo (Ctrl+Shift+Z)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
        Redo
      </button>
      <button class="ghost" onclick={() => void revert()} disabled={!dirty || loading} title="Revert to original">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
        Reset
      </button>
      <span class="separator"></span>
      <button class="ghost" onclick={() => void copy()} disabled={loading || saving} title="Copy to clipboard (Ctrl+C)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        Copy
      </button>
      <button class="primary" onclick={() => void save()} disabled={loading || saving} title="Save and close (Ctrl+S)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>
        {saving ? 'Saving…' : 'Save & Close'}
      </button>
      <button class="ghost" onclick={close} title="Close editor">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    </div>
  </header>

  <div class="editor-body">
    <div class="toolbar-strip" role="toolbar" aria-label="Annotation tools">
      {#each tools as item (item.id)}
        <button
          class="tool-btn"
          class:active={tool === item.id}
          title={item.label}
          onclick={() => selectTool(item.id)}
        >
          <span class="tool-icon">{item.icon}</span>
        </button>
      {/each}
    </div>

    <div class="canvas-area">
      <div class="zoom-bar">
        <button class="zoom-btn" onclick={zoomOut} title="Zoom out">−</button>
        <button class="zoom-btn zoom-label" onclick={fitZoom} title="Fit to window">{zoomLabel}</button>
        <button class="zoom-btn" onclick={zoomIn} title="Zoom in">+</button>
        <button class="zoom-btn" onclick={zoomActual} title="Actual size (100%)">1:1</button>
        <span class="tool-hint">{toolHints[tool]}</span>
      </div>
      <div class="stage" bind:this={stageEl}>
        <canvas bind:this={canvasEl}></canvas>
        {#if loading}
          <div class="overlay">Loading image…</div>
        {/if}
      </div>
    </div>

    <div class="options-strip">
      <div class="options-section">
        <span class="section-label">Color</span>
        <div class="color-row">
          <input type="color" bind:value={color} onchange={applyToolMode} />
          <div class="preset-colors">
            <button class="swatch" style="background:#ff3b30" onclick={() => { color = '#ff3b30'; applyToolMode(); }} title="Red"></button>
            <button class="swatch" style="background:#ff9500" onclick={() => { color = '#ff9500'; applyToolMode(); }} title="Orange"></button>
            <button class="swatch" style="background:#ffcc00" onclick={() => { color = '#ffcc00'; applyToolMode(); }} title="Yellow"></button>
            <button class="swatch" style="background:#34c759" onclick={() => { color = '#34c759'; applyToolMode(); }} title="Green"></button>
            <button class="swatch" style="background:#007aff" onclick={() => { color = '#007aff'; applyToolMode(); }} title="Blue"></button>
            <button class="swatch" style="background:#af52de" onclick={() => { color = '#af52de'; applyToolMode(); }} title="Purple"></button>
            <button class="swatch" style="background:#ffffff" onclick={() => { color = '#ffffff'; applyToolMode(); }} title="White"></button>
            <button class="swatch" style="background:#000000" onclick={() => { color = '#000000'; applyToolMode(); }} title="Black"></button>
          </div>
        </div>
      </div>

      <div class="options-section">
        <span class="section-label">Size — {strokeWidth}px</span>
        <input class="range-fixed" type="range" min="1" max="24" bind:value={strokeWidth} onchange={applyToolMode} />
        <div class="size-presets">
          <button class="size-preset" class:active={strokeWidth === 2} onclick={() => { strokeWidth = 2; applyToolMode(); }}>S</button>
          <button class="size-preset" class:active={strokeWidth === 4} onclick={() => { strokeWidth = 4; applyToolMode(); }}>M</button>
          <button class="size-preset" class:active={strokeWidth === 8} onclick={() => { strokeWidth = 8; applyToolMode(); }}>L</button>
          <button class="size-preset" class:active={strokeWidth === 16} onclick={() => { strokeWidth = 16; applyToolMode(); }}>XL</button>
        </div>
      </div>

      {#if tool === 'text'}
        <div class="options-section">
          <span class="section-label">Text</span>
          <input class="text-fluid" type="text" bind:value={text} placeholder="Enter text…" />
        </div>
      {/if}

      {#if tool === 'emoji'}
        <div class="options-section">
          <span class="section-label">Emoji</span>
          <div class="emoji-row">
            {#each EMOJIS as e (e)}
              <button class="emoji-btn" class:active={emojiChoice === e} onclick={() => (emojiChoice = e)}>{e}</button>
            {/each}
          </div>
        </div>
      {/if}

      {#if tool === 'image'}
        <div class="options-section">
          <span class="section-label">Image</span>
          <span class="hint-inline">Click the canvas to choose a file</span>
        </div>
      {/if}

      {#if tool === 'balloon'}
        <div class="options-section">
          <span class="section-label">Balloon</span>
          <input class="text-fluid" type="text" bind:value={balloonText} placeholder="Enter balloon text…" />
        </div>
      {/if}

      {#if tool === 'magnify'}
        <div class="options-section">
          <span class="section-label">Zoom — {magnifyZoom}x</span>
          <input class="range-fixed" type="range" min="1.5" max="5" step="0.5" bind:value={magnifyZoom} />
        </div>
      {/if}

      {#if tool === 'blur' || tool === 'pixelate'}
        <div class="options-section">
          <span class="section-label">Intensity</span>
          <input class="range-fixed" type="range" min="1" max="24" bind:value={strokeWidth} />
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

      {#if tool === 'rect'}
        <div class="options-section">
          <span class="section-label">Corners — {cornerRadius}px</span>
          <input class="range-fixed" type="range" min="0" max="48" bind:value={cornerRadius} />
        </div>
      {/if}

      {#if tool === 'rect' || tool === 'ellipse' || tool === 'line' || tool === 'arrow' || tool === 'text'}
        <div class="options-section">
          <span class="section-label">Shadow</span>
          <label class="toggle-row">
            <input type="checkbox" bind:checked={shapeShadow} />
            <span>Drop shadow</span>
          </label>
        </div>
      {/if}

      {#if tool === 'line'}
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
          <span class="section-label">Font size — {fontSize}px</span>
          <input class="range-fixed" type="range" min="10" max="96" bind:value={fontSize} />
          {#if tool === 'text'}
            <label class="toggle-row">
              <input type="checkbox" bind:checked={fontBold} />
              <span>Bold</span>
            </label>
            <label class="toggle-row">
              <input type="checkbox" bind:checked={fontItalic} />
              <span>Italic</span>
            </label>
            <span class="section-label">Alignment</span>
            <div class="line-styles">
              <button class="line-btn" class:active={textAlign === 'left'} onclick={() => (textAlign = 'left')} title="Left">⇤</button>
              <button class="line-btn" class:active={textAlign === 'center'} onclick={() => (textAlign = 'center')} title="Center">⇔</button>
              <button class="line-btn" class:active={textAlign === 'right'} onclick={() => (textAlign = 'right')} title="Right">⇥</button>
            </div>
          {/if}
        </div>
      {/if}

      {#if tool !== 'crop' && tool !== 'cursor' && tool !== 'cutout' && tool !== 'select'}
        <div class="options-section">
          <span class="section-label">Opacity — {opacity}%</span>
          <input class="range-fixed" type="range" min="10" max="100" bind:value={opacity} />
        </div>
      {/if}
    </div>
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
    white-space: nowrap;
  }
  .title-group {
    min-width: 0;
    margin-right: 0.6rem;
  }
  .sub {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-shrink: 0;
  }
  /* Icon-only header buttons on narrow windows so nothing (incl. ✕) clips. */
  @media (max-width: 1100px) {
    .head-actions {
      gap: 0.25rem;
    }
    .head-actions button {
      font-size: 0;
      gap: 0;
      padding: 0.4rem 0.45rem;
    }
    .head-actions button svg {
      width: 16px;
      height: 16px;
    }
    .sub {
      display: none;
    }
  }
  .separator {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 0.3rem;
  }

  /* ---- Body: 3-column layout ---- */
  .editor-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  /* ---- Top toolbar strip (compact quick-edit dialog) ---- */
  .toolbar-strip {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 5px 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    flex-shrink: 0;
    scrollbar-width: thin;
  }
  .tool-btn {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex-shrink: 0;
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
    font-size: 1.05rem;
    line-height: 1;
  }

  /* ---- Canvas area ---- */
  .canvas-area {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }
  .zoom-bar {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.35rem 0.6rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .zoom-btn {
    padding: 0.15rem 0.55rem;
    font-size: 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
  }
  .zoom-btn:hover {
    border-color: var(--accent);
  }
  .zoom-label {
    min-width: 3.2rem;
    font-variant-numeric: tabular-nums;
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

  /* ---- Bottom options strip ---- */
  .options-strip {
    background: var(--surface);
    border-top: 1px solid var(--border);
    padding: 0.4rem 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.4rem 1.1rem;
    flex-wrap: nowrap;
    overflow-x: auto;
    flex-shrink: 0;
    scrollbar-width: thin;
    min-height: 54px;
  }
  .options-section {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-shrink: 0;
  }
  .options-section .section-label {
    text-align: left;
    padding: 0;
    white-space: nowrap;
  }
  .section-label {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    padding: 0.2rem 0;
  }
  .color-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .color-row input[type='color'] {
    width: 34px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-3);
    cursor: pointer;
    flex-shrink: 0;
  }
  .preset-colors {
    display: flex;
    gap: 0.25rem;
    flex-wrap: nowrap;
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
    width: 110px;
    accent-color: var(--accent);
    flex-shrink: 0;
  }
  .options-section input[type='range'].range-fixed {
    width: 110px;
  }
  .size-presets {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }
  .size-preset {
    padding: 0.25rem 0.5rem;
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
    width: 150px;
    padding: 0.35rem 0.5rem;
    border-radius: 7px;
    border: 1px solid var(--border);
    background: var(--surface-3);
    color: var(--text);
    font-size: 0.8rem;
    flex-shrink: 0;
  }
  .options-section input[type='text'].text-fluid {
    width: clamp(140px, 18vw, 240px);
  }
  .options-section input[type='text']:focus {
    outline: 2px solid color-mix(in srgb, var(--accent) 60%, transparent);
    outline-offset: 1px;
  }
  .hint-inline {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-dim);
    white-space: nowrap;
  }
  .tool-hint {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-left: 0.8rem;
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
  .emoji-row {
    display: flex;
    gap: 0.15rem;
    max-width: 300px;
    overflow-x: auto;
    scrollbar-width: thin;
  }
  .emoji-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    font-size: 1.05rem;
    padding: 0;
    border-radius: 7px;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
  }
  .emoji-btn:hover {
    background: var(--surface-2);
  }
  .emoji-btn.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
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
    padding: 0.3rem 0.45rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface-3);
    color: var(--text);
    font-size: 0.76rem;
    max-width: 130px;
    flex-shrink: 0;
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
  button.active-tool {
    border-color: var(--accent);
    color: var(--accent);
  }
  .zoom-bar .zoom-btn {
    display: inline-flex;
  }
</style>
