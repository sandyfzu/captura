# xshot

Cross-platform screen capture for Node.js — a high-performance native module with a fully typed async TypeScript API.

Built with [Rust](https://www.rust-lang.org/), [xcap](https://github.com/nashaofu/xcap), and [napi-rs](https://napi.rs/).

## Features

- **Cross-platform** — macOS (x64, arm64), Windows (x64, arm64), Linux (x64, arm64 — glibc and musl)
- **Async/Promise-based** — every function returns a `Promise` and never blocks the Node.js event loop
- **Fully typed** — auto-generated TypeScript definitions with rich JSDoc documentation
- **Multi-format** — PNG (default), JPEG, WebP, and AVIF output
- **Buffer & Base64** — get screenshots as a Node.js `Buffer` or a Base64 string
- **Physical & logical coordinates** — monitor metadata exposes both pixel-exact and DIP geometry

## Requirements

- **Node.js** >= 20.3.0 (N-API 9)

## Installation

```bash
npm install xshot
```

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
|----------|---------|-------------|
| `getMonitors()` | `Promise<Monitor[]>` | Metadata for all connected monitors |
| `getMonitorById(id)` | `Promise<Monitor>` | Metadata for a single monitor (throws `MONITOR_NOT_FOUND`) |
| `captureMonitor(id, format?)` | `Promise<CaptureResult>` | Screenshot as a `Buffer` |
| `captureAllMonitors(format?)` | `Promise<CaptureResult[]>` | Screenshots of every monitor |
| `captureMonitorBase64(id, format?)` | `Promise<Base64CaptureResult>` | Screenshot as a Base64 string |
| `captureAllMonitorsBase64(format?)` | `Promise<Base64CaptureResult[]>` | Base64 screenshots of every monitor |

### Image Formats

The optional `format` parameter accepts (case-insensitive):

| Value | MIME Type | Notes |
|-------|-----------|-------|
| `'Png'` | `image/png` | **Default.** Lossless, pixel-perfect. |
| `'Jpeg'` / `'Jpg'` | `image/jpeg` | Lossy, default quality. |
| `'WebP'` | `image/webp` | Lossless only. |
| `'Avif'` | `image/avif` | Default speed and quality. |

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
  format: ImageFormat      // 'Png' | 'Jpeg' | 'WebP' | 'Avif'
  data: Buffer
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

### Error Handling

All errors are standard JavaScript `Error` objects with a `code` string for programmatic matching:

```ts
try {
  await captureMonitor(999)
} catch (err) {
  // err.message: "[MONITOR_NOT_FOUND] Monitor not found: no monitor with id 999"
}
```

| Error Code | Description |
|------------|-------------|
| `INITIALIZATION_ERROR` | Failure during module initialisation |
| `MONITOR_NOT_FOUND` | Requested monitor ID does not exist |
| `CAPTURE_FAILED` | Screenshot operation failed |
| `PERMISSION_DENIED` | OS denied screen capture permission |
| `PLATFORM_NOT_SUPPORTED` | Feature unavailable on this OS |
| `ENCODING_ERROR` | Image encoding failure |
| `INVALID_ARGUMENT` | Invalid parameter (e.g. bad format string) |
| `INTERNAL_ERROR` | Unexpected internal failure |
| `TIMEOUT_ERROR` | Operation exceeded time bounds |
| `RESOURCE_UNAVAILABLE` | OS resource became unavailable |

## Platform Notes

- **macOS** — Screen recording permission is required. The OS will prompt on first use. If denied, functions throw `PERMISSION_DENIED`.
- **Linux** — Supports X11 and Wayland (via xcap). Some Wayland compositors may have limited support.
- **Windows** — No special permissions required.

## License

[MIT](LICENSE)
