# ShareV Roadmap

Current status: **Milestone 11 — Auto Capture & Monitoring** (complete)

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
