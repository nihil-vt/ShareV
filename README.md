# ShareV

A ShareX-style screen capture and sharing tool for Linux, built with
[Tauri 2](https://tauri.app) + [Svelte 5](https://svelte.dev).

> Status: **milestone 11 — auto capture & monitoring**. Capture a region, the
> fullscreen, a specific monitor, or a specific window; pick a delay before the
> shot; run a chain of tasks after each capture (save, copy, upload, pin to
> screen, open the editor, run a command, notify, OCR); apply a post-capture
> image-effects pipeline; stitch long pages with scrolling capture; capture on a
> timer with change detection; and record the screen to MP4, WebM or GIF. Global
> hotkeys work on Wayland through the XDG portal.

## Features (current)

- **Region / Fullscreen / Window capture**, with a **delay timer** and a
  **monitor picker**
- **Window picker** to capture one specific window (Hyprland, Sway, X11)
- **Scrolling capture** — select a region, scroll, and ShareV stitches the
  frames into a single tall image (overlap detection + row-signature matching)
- **Auto capture** — grab a locked region (or the screen) on an interval, save
  only when the content changes, and stop after N captures or a time limit
  (`sharev capture auto`); live controls in the Capture view
- **Task chains** — ordered post-capture and post-upload tasks (save, history,
  copy, editor, open, upload, pin, command, notify, OCR / copy link, notify,
  open), editable in Settings with automatic migration from the legacy flags
- **OCR** — extract text with Tesseract (`sharev ocr <path>`, the **Run OCR**
  task, or the editor), copied straight to the clipboard; configurable language
- **Image effects** — shadow, border, watermark, canvas padding, blur and
  rounded corners, applied in sequence after saving
- **Pin-to-screen** — keep a capture floating in a borderless, always-on-top,
  transparent window
- **Annotation editor** — rectangle, ellipse, arrow, pen, text, speech balloon,
  magnifier, spotlight, cursor, pixelate, blur and crop tools with colour, fill,
  line-style, opacity, font and arrowhead controls and undo/redo
- **Screen recording** — MP4/H.264, WebM/VP9 or animated GIF, with configurable
  frame rate and optional system-audio and/or microphone capture
- **Save** to a configurable directory with a strftime filename template
- **PNG / JPEG / WebP** output with configurable quality
- **Clipboard** copy (`wl-copy` on Wayland, `xclip` on X11)
- **Uploads / sharing** — Imgur (anonymous), a generic HTTP `multipart` POST,
  or a custom shell command; the returned link is copied as a plain URL,
  Markdown or HTML
- **History** with thumbnails, search and per-item actions (open, copy, edit,
  upload, remove) plus the saved upload URL
- **System tray** menu with capture, scrolling capture, recording toggle,
  pin-last and upload-last
- **Global hotkeys** on Wayland (XDG portal) and X11
  (`tauri-plugin-global-shortcut`), with a binding editor in Settings
- **Headless CLI** for compositor bindings, with `--delay`, `--monitor` and
  `--window` targeting
- **Installable packages** — `.deb`, `.rpm` and AppImage, with signed in-app
  auto-updates from GitHub Releases
- **Backend abstraction** — wlroots (`grim` + `slurp`) now, X11
  (`import`) now, XDG portal (`ashpd`) stubbed for later

## Requirements

Runtime tools (at least one capture backend must be present):

| Backend   | Session        | Tools                         |
| --------- | -------------- | ----------------------------- |
| `wlroots` | Wayland wlroots| `grim`, `slurp`               |
| `x11`     | X11 (or XWayland) | ImageMagick `import`, `xdotool` (window) |
| `portal`  | any Wayland    | not implemented yet           |

Clipboard: `wl-clipboard` (`wl-copy`) on Wayland or `xclip` on X11.

**Optional features are toggleable in the config (`[features]`) and never
require their dependency to be installed while disabled** — enabling a feature
whose binary is missing shows the install command for your distribution (apt /
dnf / pacman / zypper / apk / xbps) in Settings and in the CLI output.

- OCR (feature `ocr`, default on, needs `tesseract` plus any extra language
  packs you want, e.g. `tesseract-data-fra`).

Window and monitor enumeration needs `hyprctl` (Hyprland), `swaymsg` (Sway), or
`wmctrl` + `xrandr` (X11).

Recording needs `wf-recorder` on Wayland/wlroots or `ffmpeg` on X11
(`x11grab` + `pulse`); GIF output always transcodes through `ffmpeg`. Fullscreen
recording on wlroots needs `hyprctl` (Hyprland), `swaymsg` (Sway), or a single
enabled output via `wlr-randr`. Audio device listing uses `pactl`.

## Build & run

```bash
bun install

# Development (recommended: starts Vite + the Tauri window)
bun run tauri dev

# Just the frontend in a browser
bun run dev
```

> **WebKitGTK blank/crashed window on Wayland?** Some WebKitGTK builds fail
> with `Error 71 (Protocol error) dispatching to Wayland display` when the
> DMABUF renderer is enabled. Launch with
> `WEBKIT_DISABLE_DMABUF_RENDERER=1 bun run tauri dev` to work around it.

Produce a release bundle:

```bash
bun run tauri build
```

This writes `.deb`, `.rpm` and `.AppImage` bundles (plus signed updater
artifacts) under `src-tauri/target/release/bundle/`.

## Install

Prebuilt packages are published on the
[Releases](https://github.com/nihil-vt/ShareV/releases) page:

- **`.deb`** — Debian / Ubuntu: `sudo apt install ./ShareV_*_amd64.deb`
- **`.rpm`** — Fedora / openSUSE: `sudo dnf install ./ShareV-*.x86_64.rpm`
- **`.AppImage`** — any distribution:
  `chmod +x ShareV_*_amd64.AppImage && ./ShareV_*_amd64.AppImage`

External tools (see [Requirements](#requirements)) are not bundled; the `.deb`
and `.rpm` declare the most common ones as recommendations, but the AppImage
needs them installed on the host.

Check for updates from **Settings → Updates**. Updates are downloaded from the
[Releases](https://github.com/nihil-vt/ShareV/releases) page, verified against a
signing key, and applied on restart.

## Releasing

The version lives in `package.json`, `src-tauri/Cargo.toml` and
`src-tauri/tauri.conf.json`. Bump them together:

```bash
bun run bump 0.2.0
```

Then commit, tag and push:

```bash
git commit -am "Release v0.2.0"
git tag v0.2.0
git push origin main --tags
```

The `Release` GitHub Actions workflow builds the `.deb`, `.rpm` and AppImage
bundles, signs the updater artifacts, and opens a **draft** release. Publish the
draft once the assets look right.

Signing keys are created with `bun run tauri signer generate`. The public key
lives in `tauri.conf.json`; the private key and its password are stored as
repository secrets (`TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Cross-version updates only work when the
app bundle is signed with the matching private key.

Updater artifacts are only produced during releases: the workflow passes
`--config src-tauri/tauri.updater.conf.json` to enable signed artifacts, so
local `bun run tauri build` runs without needing a private key.

## CLI

ShareV doubles as a small CLI — useful for compositor bindings, or as a
fallback when hotkeys are unavailable:

```bash
sharev capture region        # select a region, prints the saved path
sharev capture fullscreen
sharev capture window
sharev capture scrolling     # scroll a region, stitch it into one image
sharev capture auto          # capture on an interval (Ctrl-C to stop)
sharev capture auto --interval 500 --change --max-count 20
sharev capture fullscreen --monitor HDMI-1 --delay 3
sharev capture window --window 0x123
sharev upload shot.png       # upload a file, prints the URL (and copies the link)
sharev ocr shot.png          # run OCR, print the text (and copy it)
sharev record region         # record until Ctrl-C, then print the path
sharev record fullscreen
sharev record window
sharev pin shot.png          # keep a capture floating on screen
sharev diagnostics           # environment + backend report
sharev --help
```

Global hotkeys are built in — on Wayland they register through the XDG portal,
on X11 through `tauri-plugin-global-shortcut`. If neither works (e.g. the portal
is unavailable on X11), bind the CLI in your compositor instead. Example
Hyprland config (`~/.config/hypr/hyprland.conf`):

```conf
bind = , PRINT,      exec, sharev capture region
bind = SHIFT, PRINT, exec, sharev capture fullscreen
bind = ALT, PRINT,   exec, sharev capture window
bind = SUPER SHIFT, R, exec, sharev record region
```

`sharev record` starts recording immediately and stops on Ctrl-C, so bind it as
above and press the key again, or wrap it in your own toggle script.

## Configuration

Written to `~/.config/sharev/config.toml` on first run. History lives in
`~/.local/share/sharev/history.json`.

## Window manager integration

The annotation editor opens in a small dialog window titled
**ShareV Editor**. On Hyprland it floats itself automatically — no setup
needed. Everywhere else, add a one-line rule to keep it floating instead of
tiled (used automatically as a fallback where supported):

```ini
# Hyprland (hyprland.conf) — only needed on old releases without Lua dispatch
windowrulev2 = float, title:^(ShareV Editor)$
windowrulev2 = center, title:^(ShareV Editor)$
windowrulev2 = size 880 620, title:^(ShareV Editor)$
```

```ini
# Sway / i3 (config)
for_window [title="ShareV Editor"] floating enable
```

## Uploading

Pick a destination under **Settings → Uploads**. Three uploaders are
available, and the returned link is copied to the clipboard as a plain URL,
Markdown image, or HTML tag (your choice).

- **Imgur (anonymous)** — paste a free Client-ID from
  `https://api.imgur.com/oauth2/addclient`. No account or OAuth needed.
- **Custom HTTP endpoint** — a `multipart/form-data` POST. Configure the URL,
  the file field name (default `file`), extra headers (one `Name: Value` per
  line), and a dotted JSON path to the URL in the response (e.g. `data.link`;
  leave blank to use the raw response body as the URL).
- **Custom command** — run any shell command. The file path is substituted for
  `{file}` (or appended), and is also exported as `$SHAREV_FILE`. The last
  non-empty line of stdout is used as the URL.

The same destinations can be driven from the CLI with
`sharev upload <path>`.

## Recording

Open **Record** and pick a region, the fullscreen, or the focused window. While
recording, a timer and a stop button are shown; recordings are added to
**History** like captures. Configure the format and audio under
**Settings → Recording**:

- **Format** — MP4/H.264, WebM/VP9, or animated GIF. GIFs are recorded to a
  temporary video and transcoded with `ffmpeg` when you stop.
- **Frame rate** — capture fps, plus a separate GIF fps.
- **Audio** — none, system, microphone, or both. System and microphone devices
  are chosen independently from the PulseAudio/PipeWire sources reported by
  `pactl` (empty = the default sink monitor / source).

On Wayland, `wf-recorder` records a single output, so fullscreen recording
targets the focused monitor. From the CLI, run `sharev record <kind>` and press
Ctrl-C to stop.

## Project layout

```
src/                       Svelte 5 frontend (runes)
  lib/components/          Sidebar, Capture/Record/History/Editor/Settings/Diagnostics/Pin views
  lib/api.ts               typed command wrappers
  lib/state.svelte.ts      reactive app state
src-tauri/src/
  capture/                 CaptureBackend trait + wlroots / x11 / portal
  clipboard.rs             image & text clipboard
  config.rs               TOML config model
  history.rs              capture history store
  pipeline.rs             capture -> encode -> save -> clipboard -> history
  tasks.rs                post-capture / post-upload task execution
  effects.rs              shadow / border / watermark / padding / blur / corners
  ocr.rs                  Tesseract text recognition
  scroll.rs               scrolling-capture frame stitching
  auto.rs                 timed / change-detection auto capture
  record.rs               wf-recorder / ffmpeg screen recording
  upload.rs               Imgur / HTTP / command uploaders
  commands.rs             Tauri commands
  tray.rs / shortcuts.rs  tray icon + global hotkeys
  portal_shortcuts.rs     XDG portal GlobalShortcuts (ashpd)
  pin.rs                  pin-to-screen window registry
  editor.rs               annotation-editor window registry
  wm.rs                   window / monitor enumeration
  cli.rs                  headless command line
```
