# ShareV Roadmap

Current status: **Milestone 11 — Auto Capture & Monitoring** (complete)

Milestones 1–6 shipped before this file existed; they are recorded here as
complete for reference.

## Milestone 1 — Core Capture Engine

**Goal:** A dependable screenshot core with a backend abstraction.

| Feature | Description | Status |
|---------|-------------|--------|
| Backend abstraction | `CaptureBackend` trait (wlroots / X11 / portal stub) | Done |
| Region capture | Interactive region select (`slurp` / ImageMagick `import`) | Done |
| Fullscreen capture | Capture the whole screen | Done |
| Clipboard | Copy image / text (`wl-copy` / `xclip`) | Done |
| Output formats | PNG / JPEG / WebP with configurable quality | Done |
| History | JSON history store with search | Done |
| Config | TOML config with sensible defaults | Done |
| CLI | `sharev capture`, `sharev diagnostics` | Done |

## Milestone 2 — Annotation Editor

**Goal:** Annotate captures before saving or sharing.

| Feature | Description | Status |
|---------|-------------|--------|
| Basic tools | Rectangle, ellipse, arrow, pen, text | Done |
| Highlighter | Semi-transparent highlight stroke | Done |
| Blur / pixelate | Obscure sensitive areas | Done |
| Crop | Trim the captured canvas | Done |
| Undo / redo | Undo/redo history stack | Done |
| Save / copy | Apply edits, re-save and copy | Done |

## Milestone 3 — Uploading & Sharing

**Goal:** Get captures off the machine quickly.

| Feature | Description | Status |
|---------|-------------|--------|
| Imgur uploader | Anonymous upload via Client-ID | Done |
| HTTP uploader | `multipart/form-data` POST with headers + JSON path | Done |
| Command uploader | Custom shell command (`{file}` / `$SHAREV_FILE`) | Done |
| Copy formats | Copy link as URL / Markdown / HTML | Done |
| CLI | `sharev upload <path>` | Done |

## Milestone 4 — Screen Recording

**Goal:** Record the screen to a shareable video or GIF.

| Feature | Description | Status |
|---------|-------------|--------|
| Recording backends | `wf-recorder` (Wayland), `ffmpeg` (X11) | Done |
| Formats | MP4/H.264, WebM/VP9, animated GIF | Done |
| Audio | Optional system and/or microphone capture | Done |
| Target selection | Region, fullscreen or focused window | Done |
| History | Recordings stored in the same history | Done |
| CLI | `sharev record <kind>` | Done |

## Milestone 5 — Packaging, CI & Auto-Updates

**Goal:** Distributable builds and a release pipeline.

| Feature | Description | Status |
|---------|-------------|--------|
| Packages | `.deb`, `.rpm` and AppImage bundles | Done |
| CI | Checks + tests workflow | Done |
| Release workflow | Tagged builds producing a draft release | Done |
| Auto-updates | Signed in-app updates from GitHub Releases | Done |
| Version bump | `bun run bump <version>` | Done |

## Milestone 6 — Capture Workflow, Tasks & Hotkeys

**Goal:** ShareX-style capture flow and global access.

| Feature | Description | Status |
|---------|-------------|--------|
| Delay timer | Configurable pre-capture delay | Done |
| Monitor picker | Choose a monitor for fullscreen capture | Done |
| Window picker | Capture a specific window | Done |
| Task chains | Ordered post-capture / post-upload tasks | Done |
| Pin-to-screen | Borderless always-on-top floating capture | Done |
| Wayland hotkeys | XDG portal GlobalShortcuts (`ashpd`) | Done |
| X11 hotkeys | `tauri-plugin-global-shortcut` | Done |
| Hotkey editor | Binding editor with backend selection | Done |
| Dynamic tray | Start/Stop recording, pin/upload last | Done |

## Milestone 7 — Annotation & Editing

**Goal:** Match ShareX's editor toolset for screenshot annotation.

| Feature | Description | Status |
|---------|-------------|--------|
| Speech balloons | Rounded callout with tail pointer | Done |
| Magnifier | Enlarge a region while keeping context | Done |
| Spotlight | Dim everything except a highlighted region | Done |
| Cursor insertion | Place a cursor graphic at a position | Done |
| Image insertion | Embed a second image onto the canvas | Planned |
| Cut-out | Remove a region and make it transparent | Done |
| Line style | Solid, dashed, dotted stroke options | Done |
| Shape fill | Optional fill color for rectangles/ellipses | Done |
| Opacity control | Per-shape opacity slider | Done |
| Font family | Configurable font for text tool | Done |
| Arrow styles | Start/end/both arrowheads | Done |

## Milestone 8 — OCR & Text Recognition

**Goal:** Extract text from screenshots and make it actionable.

| Feature | Description | Status |
|---------|-------------|--------|
| OCR engine | Integrate Tesseract or similar | Done |
| OCR command | `sharev ocr <path>` CLI command | Done |
| Auto-OCR after capture | Optional task in the capture chain | Done |
| Copy to clipboard | OCR result copied automatically | Done |
| OCR language | Configurable language setting | Done |
| Text overlay | Show recognized text over image | Planned |

## Milestone 9 — Image Effects

**Goal:** Post-capture effects pipeline like ShareX.

| Feature | Description | Status |
|---------|-------------|--------|
| Shadow | Drop shadow with configurable offset/blur | Done |
| Border | Solid/gradient border around image | Done (solid) |
| Watermark | Text or image watermark overlay | Done (text) |
| Canvas resize | Add padding around the image | Done |
| Blur effect | Gaussian blur on entire image or region | Done (whole image) |
| Rounded corners | Apply border radius to the image | Done |
| Image beautifier | Auto-enhance captures | Planned |
| Effect chains | Multiple effects applied in sequence | Done |

## Milestone 10 — Scrolling Capture

**Goal:** Capture scrolling content by stitching screenshots.

| Feature | Description | Status |
|---------|-------------|--------|
| Scroll detection | Detect scrollable regions | Partial (detects when scrolling stops) |
| Auto-scroll | Programmatic scroll via compositor IPC | Planned |
| Stitch algorithm | Overlap detection and merging | Done |
| Manual mode | User scrolls, app captures each frame | Done |
| Output | Single tall image of full content | Done |
| CLI support | `sharev capture scrolling` command | Done |

## Milestone 11 — Auto Capture & Monitoring

**Goal:** Timed and event-driven automatic captures.

| Feature | Description | Status |
|---------|-------------|--------|
| Interval capture | Capture every N seconds | Done |
| Region lock | Always capture the same region | Done |
| Change detection | Only save when content changed | Done |
| Max count / time limit | Stop after N captures or M minutes | Done |
| CLI support | `sharev capture auto` command | Done |
| Monitoring UI | Live count + start/stop controls | Done |

## Milestone 12 — Advanced Capture Features

**Goal:** Remaining ShareX capture features.

| Feature | Description | Status |
|---------|-------------|--------|
| Multi-region capture | Select multiple disjoint regions | Planned |
| Fixed size | Lock region to preset dimensions | Planned |
| Monitor test | Test pattern for monitor calibration | Planned |
| Window inspector | Inspect window properties | Planned |
| Color picker | Pick colors from screen | Planned |
| Ruler | On-screen pixel ruler | Planned |

## Milestone 13 — Integrations & Tools

**Goal:** Additional ShareX tools beyond capture.

| Feature | Description | Status |
|---------|-------------|--------|
| Hash checker | Verify file integrity (MD5/SHA) | Planned |
| QR code generator | Generate QR codes from text/URL | Planned |
| QR code scanner | Read QR codes from screenshots | Planned |
| Clipboard viewer | View clipboard history | Planned |
| Image comparer | Side-by-side image diff | Planned |
| Image combiner | Merge multiple images | Planned |
| Image splitter | Split image into tiles | Planned |
| Metadata viewer | View image EXIF/metadata | Planned |

## Milestone 14 — Polish & Distribution

**Goal:** Final polish and broader distribution.

| Feature | Description | Status |
|---------|-------------|--------|
| Theme system | Light/dark/custom themes | Planned |
| Plugin system | Community extensions | Planned |
| Flatpak packaging | Flathub submission | Planned |
| AUR package | Arch Linux AUR package | Planned |
| Performance | Startup time optimization | Planned |
| Accessibility | Screen reader support | Planned |
| i18n | Internationalization | Planned |
