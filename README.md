# ShareV

A ShareX-style screen capture and sharing tool for Linux, built with
[Tauri 2](https://tauri.app) + [Svelte 5](https://svelte.dev).

> Status: **milestone 4 — screen recording**. Capture a region, the
> fullscreen, or the focused window; annotate the result with shapes, arrows,
> a pen, text, pixelation and cropping; then save it, copy it to the clipboard,
> upload it to a destination, and find it again in a searchable history. Record
> the screen to MP4, WebM or GIF with optional system and microphone audio. A
> system tray and best-effort global hotkeys are included. OCR and the rest of
> ShareX's toolset come next.

## Features (current)

- **Region / Fullscreen / Window capture**
- **Annotation editor** — rectangle, ellipse, arrow, pen, text, pixelate and
  crop tools with colour/width controls and undo/redo
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
- **System tray** menu
- **Global hotkeys** (X11) and a **headless CLI** for compositor bindings
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

Window capture on wlroots needs `hyprctl` (Hyprland) or `swaymsg` (Sway).

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
cd src-tauri && cargo build --release
```

## CLI

ShareV doubles as a small CLI — the recommended way to bind hotkeys on
Wayland, where compositors grab keys themselves:

```bash
sharev capture region        # select a region, prints the saved path
sharev capture fullscreen
sharev capture window
sharev upload shot.png       # upload a file, prints the URL (and copies the link)
sharev record region         # record until Ctrl-C, then print the path
sharev record fullscreen
sharev record window
sharev diagnostics           # environment + backend report
sharev --help
```

Example Hyprland config (`~/.config/hypr/hyprland.conf`):

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
  lib/components/          Sidebar, Capture/Record/History/Editor/Settings/Diagnostics views
  lib/api.ts               typed command wrappers
  lib/state.svelte.ts      reactive app state
src-tauri/src/
  capture/                 CaptureBackend trait + wlroots / x11 / portal
  clipboard.rs             image & text clipboard
  config.rs               TOML config model
  history.rs              capture history store
  pipeline.rs             capture -> encode -> save -> clipboard -> history
  record.rs               wf-recorder / ffmpeg screen recording
  upload.rs               Imgur / HTTP / command uploaders
  commands.rs             Tauri commands
  tray.rs / shortcuts.rs  tray icon + global hotkeys
  cli.rs                  headless command line
```
