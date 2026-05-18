# xshot

[![CI](https://github.com/sandyfzu/xshot/actions/workflows/ci.yml/badge.svg)](https://github.com/sandyfzu/xshot/actions/workflows/ci.yml)

Cross-platform screen capture for Node.js — a high-performance native module built with [Rust](https://www.rust-lang.org/) and a fully typed async TypeScript API.

## Features

- **Cross-platform** — macOS (x64, arm64), Windows (x64, arm64), Linux (x64, arm64 — glibc and musl)
- **Async/Promise-based** — every function returns a `Promise` and never blocks the Node.js event loop
- **Fully typed** — auto-generated TypeScript definitions with rich JSDoc documentation
- **Multi-format** — Raw (unencoded RGBA), PNG (default), JPEG, WebP, and AVIF output
- **Buffer & Base64** — get screenshots as a Node.js `Buffer` or a Base64 string
- **Physical & logical coordinates** — monitor metadata exposes both pixel-exact and DIP geometry

## Requirements

- **Node.js** >= 20.3.0 (N-API 9)

### Linux native dependencies

xshot uses native X11, Wayland, PipeWire, D-Bus, EGL, and GBM libraries on
Linux. Desktop Ubuntu installations usually include many of these libraries
already, but minimal images and containers may need them installed before the
native addon can load or before Wayland capture can talk to the desktop portal
services.

For Ubuntu 24.04 runtime installations, install:

```bash
sudo apt-get update
sudo apt-get install -y \
  libxcb1 libxrandr2 libdbus-1-3 \
  libpipewire-0.3-0t64 libwayland-client0 libegl1 libgbm1 \
  xdg-desktop-portal
```

On Ubuntu releases that do not use the `t64` PipeWire runtime package name,
install `libpipewire-0.3-0` instead of `libpipewire-0.3-0t64`.

If you are building xshot from source or rebuilding the native addon on Ubuntu,
install the development headers too:

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libclang-dev \
  libxcb1-dev libxrandr-dev libdbus-1-dev \
  libpipewire-0.3-dev libwayland-dev libegl-dev libgbm-dev
```

These package lists are based on xshot's Ubuntu 24.04 CI build configuration.
`libgbm-dev`/`libgbm1` are included because xshot's Wayland capture layer
requires GBM.

## Installation

```bash
npm install xshot
```

Maintainers preparing a release should follow the native package checklist in
[RELEASE.md](RELEASE.md). The published package must include the generated
platform packages expected by the NAPI-RS loader.

## Quick Start

```ts
import {
  getMonitors,
  getMonitorById,
  captureMonitor,
  captureAllMonitors,
  captureMonitorBase64,
  captureAllMonitorsBase64,
} from 'xshot'
```

### List monitors

```ts
const monitors = await getMonitors()

for (const m of monitors) {
  console.log(`${m.id}: ${m.friendlyName} (${m.physical.width}×${m.physical.height})`)
}
```

### Get a specific monitor

```ts
const monitor = await getMonitorById(1) // throws MONITOR_NOT_FOUND if invalid
```

### Capture a screenshot (Buffer)

```ts
import { writeFileSync } from 'node:fs'

// Default format — PNG
const result = await captureMonitor(1)
writeFileSync('screenshot.png', result.screenshot.data)

// Explicit format
const jpg = await captureMonitor(1, 'Jpeg')
writeFileSync('screenshot.jpg', jpg.screenshot.data)
```

### Capture raw RGBA pixels (fastest)

```ts
// 'Raw' skips image encoding entirely — it returns the RGBA8 pixel
// buffer with no compression, making it significantly faster than any
// encoded format. Use it when you plan to process the pixels yourself.
const raw = await captureMonitor(1, 'Raw')
const { width, height } = raw.screenshot.size

// Buffer layout: 4 bytes per pixel (R, G, B, A), row-major.
// Length is always width × height × 4.
console.log(`${width}×${height} — ${raw.screenshot.data.length} bytes`)

// Feed into sharp, node-canvas, WebGL textures, or re-encode
// with your own quality/format settings.
```

### Capture all monitors

```ts
const results = await captureAllMonitors()

for (const r of results) {
  writeFileSync(`${r.monitor.friendlyName}.png`, r.screenshot.data)
}
```

### Capture as Base64

```ts
const b64 = await captureMonitorBase64(1)
const dataUri = `data:image/png;base64,${b64.screenshot.data}`

// All monitors
const all = await captureAllMonitorsBase64('Avif')
```

## API

### Functions

| Function | Returns | Description |
| --- | --- | --- |
| `getMonitors()` | `Promise<Monitor[]>` | Metadata for all connected monitors |
| `getMonitorById(id)` | `Promise<Monitor>` | Metadata for a single monitor (throws `MONITOR_NOT_FOUND`) |
| `captureMonitor(id, format?)` | `Promise<CaptureResult>` | Screenshot as a `Buffer` |
| `captureAllMonitors(format?)` | `Promise<CaptureResult[]>` | Screenshots of every monitor |
| `captureMonitorBase64(id, format?)` | `Promise<Base64CaptureResult>` | Screenshot as a Base64 string |
| `captureAllMonitorsBase64(format?)` | `Promise<Base64CaptureResult[]>` | Base64 screenshots of every monitor |

### Image Formats

The optional `format` parameter accepts (case-insensitive):

| Value | MIME Type | Notes |
| --- | --- | --- |
| `'Raw'` | `application/octet-stream` | Unencoded RGBA8 pixels. **Fastest** — skips compression entirely. Not supported for Base64 functions. |
| `'Png'` | `image/png` | **Default.** Lossless, pixel-perfect. |
| `'Jpeg'` / `'Jpg'` | `image/jpeg` | Lossy, default quality. |
| `'WebP'` | `image/webp` | Lossless only. |
| `'Avif'` | `image/avif` | Default speed and quality. Usually the slowest built-in encoder. |

> **Note:** Passing `'Raw'` to `captureMonitorBase64()` or `captureAllMonitorsBase64()` throws an `INVALID_ARGUMENT` error. Raw pixel data is not self-describing and cannot be used in data URIs.

### Types

```ts
interface Monitor {
  id: number
  name: string             // System device name
  friendlyName: string     // Human-readable name
  physical: Bounds         // Geometry in physical pixels
  logical: Bounds          // Geometry in logical/DIP units
  rotation: number
  scaleFactor: number
  frequency: number
  isPrimary: boolean
  isBuiltin: boolean
}

interface Bounds {
  x: number
  y: number
  width: number
  height: number
}

interface CaptureResult {
  monitor: Monitor
  screenshot: Screenshot
}

interface Screenshot {
  size: Size
  format: ImageFormat      // 'Raw' | 'Png' | 'Jpeg' | 'WebP' | 'Avif'
  data: Buffer             // Raw RGBA8 pixels (format === 'Raw') or encoded image bytes
}

interface Base64CaptureResult {
  monitor: Monitor
  screenshot: Base64Screenshot
}

interface Base64Screenshot {
  size: Size
  format: ImageFormat
  data: string             // RFC 4648 Base64-encoded image
}

interface Size {
  width: number
  height: number
}
```

### Performance Notes

Capture and encoding work is offloaded from the Node.js main thread. xshot does
not impose a global concurrency limit across independent calls: applications
that start many high-resolution encoded captures at the same time should decide
their own queueing/backpressure policy based on their workload, latency target,
and host CPU budget. Use `'Raw'` when you need to process or encode pixels with
your own pipeline.

`captureAllMonitors()` and `captureAllMonitorsBase64()` capture monitors
sequentially inside one request to avoid unnecessary contention in the OS capture
subsystem.

### Error Handling

All failures are surfaced as JavaScript `Error` objects. The stable xshot domain
code is embedded at the start of `err.message` as a `[CODE]` prefix:

```ts
try {
  await captureMonitor(999)
} catch (err) {
  if (err instanceof Error && err.message.startsWith('[MONITOR_NOT_FOUND]')) {
    // handle missing monitor
  }
}
```

Do not rely on `err.code` for xshot domain matching in the current async API.
NAPI-RS v3 promise rejections expose the NAPI status code there; xshot keeps the
domain code in the message prefix so it remains visible and testable across all
async exports.

| Error Code | Description |
| --- | --- |
| `INITIALIZATION_ERROR` | Failure during module initialisation |
| `MONITOR_NOT_FOUND` | Requested monitor ID does not exist |
| `CAPTURE_FAILED` | Screenshot operation failed |
| `PERMISSION_DENIED` | Explicitly detected OS screen-capture permission denial |
| `PLATFORM_NOT_SUPPORTED` | Feature unavailable on this OS |
| `ENCODING_ERROR` | Image encoding failure |
| `INVALID_ARGUMENT` | Invalid parameter (e.g. bad format string) |
| `INTERNAL_ERROR` | Unexpected internal failure |
| `TIMEOUT_ERROR` | Operation exceeded time bounds |
| `RESOURCE_UNAVAILABLE` | OS resource became unavailable |

## Platform Notes

- **macOS** — Screen recording permission is required. If the OS or upstream
  capture layer reports a distinguishable permission denial, xshot surfaces
  `PERMISSION_DENIED`; otherwise the failed capture is reported as
  `CAPTURE_FAILED` with the platform error text in the message.
- **Linux** — Supports X11 and Wayland. Some Wayland compositors may have limited support.
- **Windows** — No special permissions required.

## License

[MIT](LICENSE)
