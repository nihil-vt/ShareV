# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/nihil-vt/ShareV/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nihil-vt/ShareV/releases/tag/v0.1.0
