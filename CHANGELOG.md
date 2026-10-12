# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-10-12

### Added

- **Scrolling capture** — pick a region, ShareV scrolls the page automatically
  and stitches the frames into a single tall image (overlap detection +
  row-signature matching). Trigger it from the Capture view, the tray, or
  `sharev capture scrolling [path]`.
- **Auto capture** — grab a locked region (or the full screen) on an interval,
  save only when the content changes, and stop after N captures or a time
  limit. Live controls in the Capture view, Start/Stop in the tray, and
  `sharev capture auto` with an interruptible Ctrl-C stop.
- **OCR** — extract text with Tesseract via `sharev ocr <path>`, the **Run OCR**
  capture task, or the editor's OCR button, and copy the result straight to the
  clipboard (configurable language in Settings).
- **Image effects** — shadow, border, reflection and blur applied in sequence
  after saving, configured in Settings; the history entry and `CaptureResult`
  report the final edited file (dimensions and bytes).
- **Editor toolset** — standalone editor window adding a step counter, solid /
  dashed / dotted line styles, arrowheads on either end, magnifier, spotlight,
  cursor, cutout and crop tools alongside the existing shapes and text.
- Per-source default audio devices when recording (system sink monitor vs
  microphone) with either recorder backend.

### Changed

- Captures now surface exclusively through the `capture-completed` event, so
  UI, tray and hotkey captures all flow through the same path (no duplicate
  toasts or history refreshes, and auto capture doesn't toast per frame).
- Editor saves and the image-effects task re-encode through the pipeline so the
  file bytes always match the file extension (JPEG saves flatten transparency
  correctly).
- Auto-capture stop interrupts the current interval immediately instead of
  waiting for the next one to elapse.

### Fixed

- History: writes are atomic (temp file + rename) and serialised under a lock,
  so concurrent auto-capture and UI updates can't lose entries — and a corrupt
  history file is logged instead of silently wiping all history.
- Recording could be started twice concurrently through the tray / hotkeys /
  UI race; the slot is now reserved before interactive target selection.
- Non-interactive capture failures with empty stderr are reported as errors
  instead of being misclassified as "cancelled".
- Copying a recording (or any non-image) now copies a `text/uri-list` instead
  of failing to decode it as a PNG.
- `xdg-open` children are reaped so they don't linger as zombies.
- Selecting an unavailable capture backend explicitly now gives a clear error.
- The XDG portal shortcuts session is closed when a new one replaces it.

## [0.2.0] - 2026-10-11

### Added

- **Task chains** — ordered `[afterCapture]` and `[afterUpload]` task lists
  replace the legacy boolean flags. Available tasks: save, history, copy image,
  editor, open, upload, pin, command, notify (post-capture) and copy link, notify,
  open (post-upload). Legacy configs migrate automatically.
- **Delay timer** before captures and recordings (configurable, with CLI
  `--delay`).
- **Monitor picker** for fullscreen capture and recording (configurable
  default, CLI `--monitor`, or per-capture in the UI).
- **Window picker** to capture a specific window (Hyprland, Sway, X11).
- **Pin-to-screen** — keep a capture floating in a borderless, always-on-top,
  transparent window. Available as a task, from the tray ("Pin Last Capture"),
  or via `sharev pin <path>`.
- **XDG portal global hotkeys** on Wayland via `ashpd`, with automatic fallback
  to `tauri-plugin-global-shortcut` (X11) or the compositor's own bindings.
- **Dynamic tray menu** with Start/Stop Recording, Pin Last and Upload Last.
- **Hotkey binding editor** in Settings with selectable backend (auto, portal,
  global-shortcut, none).

### Changed

- CLI: `capture`/`record` accept `--delay`, `--monitor` and `--window`.
- Settings UI: task-chain and hotkey editors replace the legacy checkboxes.
- `CaptureResult` now carries the upload URL and an `openEditor` flag.

## [0.1.0] - 2026-10-10

### Added

- Screen capture on Wayland (wlroots: `grim` + `slurp`) and X11 (ImageMagick
  `import`), with a `CaptureBackend` abstraction and an XDG portal stub.
- Annotation editor: pen, arrow, rectangle, ellipse, highlighter, text, blur and
  crop, with undo/redo.
- Uploading and sharing: anonymous Imgur, generic HTTP POST and command
  uploaders, plus URL / Markdown / HTML copy formats.
- Screen recording: MP4/H.264, WebM/VP9 and animated GIF, with system and
  microphone audio selection (`wf-recorder` on Wayland, `ffmpeg` on X11).
- Capture history at `~/.local/share/sharev/history.json` and configuration at
  `~/.config/sharev/config.toml`.
- CLI subcommands: `capture`, `record`, `upload` and `diagnostics`.
- Global hotkeys on X11 via `tauri-plugin-global-shortcut`.
- Packaging: `.deb`, `.rpm` and AppImage bundles.
- Signed in-app auto-updates backed by GitHub Releases.
- CI (checks + tests) and a tagged release workflow.

[Unreleased]: https://github.com/nihil-vt/ShareV/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nihil-vt/ShareV/releases/tag/v0.2.0
[0.1.0]: https://github.com/nihil-vt/ShareV/releases/tag/v0.1.0
