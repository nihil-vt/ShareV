# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Hash checker** tool: MD5, SHA-256, SHA-512, BLAKE3 of any file, copyable.
- **Image combiner** tool: joins images side-by-side or stacked with
  spacing and background, saved to the save directory and recorded
  in history.
- **QR code** tool: generate from text/URL (size slider) and decode one
  or more codes from an image; both copyable.
- **Image metadata** viewer: format, dimensions, byte size, mtime and
  EXIF tags, each row copyable.
- **Watch folders** tool: notify-driven per-folder watchers that land
  new image files in history with a toast and an event feed.
- **URL shortener** tool: is.gd shortening with error passthrough.
- **URL shortener** also works as an after-upload task ("Shorten URL first"):
  the copy/open/notify tasks all see the shortened link, not the raw one.
- **10 URL shortener providers** with per-provider credential forms
  (ShareX parity): 4 free (is.gd, v.gd, tinyurl, Zero Width Shortener)
  and 6 credentialed (bitly, kutt, polr, YOURLS, Firebase Dynamic
  Links, custom) — provider picker + fields in the Shorten URL dialog,
  saved in config and reused by the after-upload task. Free providers
  fail over to each other (is.gd and v.gd run on one database: a
  single outage kills both). Deliberately skipped: 2.gp, qr.net,
  nl.cm — dead/unverifiable endpoints. Note: `api.zws.im` is
  Railway-hosted and refuses TLS from some networks; its errors
  surface verbatim.
- Folder picker command backing watch folders.

### Fixed

- Main window layout collapse: a leftover global `.app` grid (220px sidebar
  track) squeezed the new layout into a narrow column and hid the history
  side. Removed all dead sidebar-era global CSS.
- Clipped controls: editor header collapses to icon-only buttons under 1100px
  (close ✕ no longer pushed out), recorder bar wraps, dropdown items ellipsis
  instead of spilling.
- Settings dialog close button moved into normal flow (was absolutely
  positioned and clipped); hotkey binding Remove buttons can't squeeze.
- Editor is a proper dialog citizen (no taskbar entry) with documented
  compositor float rules so it floats instead of tiling.
- On Hyprland the editor floats itself over IPC — zero user setup.
- Image effects grow 7 → 12 (resize, grayscale, sepia, brightness/contrast,
  rotate/flip) with settings UI and tests.
- **Screen color picker** tool (fullscreen grab without saving, click for
  HEX/RGB, keyboard accessible).

### Added
- ShareX-parity capture menu: **Last region** (replays the previous selection;
  label shows its dimensions) and a **Fullscreen on monitor…** submenu.
- **Upload** split button: Upload last capture, Upload file… (file picker),
  Copy last link.
- **Show quick task menu** after-capture task: per-capture action popup.
- Settings dialog jumps to sections (Task/Hotkey/Application settings buttons).
- **Abort recording** (discards without saving) in the recorder bar and tray;
  recorder bar shows FPS and audio mode; tray gains an auto-capture toggle.
- **Tools** split button: Run OCR on last capture, Pin last capture to screen.
- **Save as…** (save dialog) in the quick task menu and history preview.
- Tray **Enable/Disable Hotkeys** toggle.
- **Auto capture dialog** (target, interval, limits, change detection).
- **OCR image… dialog** (pick file, language, copyable result).
- History **thumbnail view** toggle; tray **Capture Delay** submenu.
- **Upload folder / text / URL** destinations (temp staging through the
  configured uploader); Upload menu and text/URL dialog.
- Recorder **pause/resume** (SIGSTOP/SIGCONT, gaps excluded from duration,
  frozen timer, tray-safe stop).
- Editor: **insert image**, **emoji stamps**, **duplicate** (Ctrl+D), text
  bold/italic/alignment, rectangle **corner radius**, **drop shadows**.
- Hotkeys grow 6 → 12 actions (last region, stop/discard recording, auto
  capture toggle, upload/pin last); tray **Image Uploader** quick-switch.

### Added

- **Feature toggles** — optional features such as OCR can be turned off in
  Settings (`[features]` in the config), so unneeded system dependencies never
  have to be installed. Enabling a feature whose dependency is missing shows
  the install command for your distribution (apt / dnf / pacman / zypper /
  apk / xbps), both in Settings and in the CLI output.

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
