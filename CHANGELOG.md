# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
