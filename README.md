# captura

[![CI](https://github.com/sandyfzu/captura/actions/workflows/ci.yml/badge.svg)](https://github.com/sandyfzu/captura/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/captura.svg)](https://www.npmjs.com/package/captura)
[![Node.js version](https://img.shields.io/node/v/captura.svg)](https://www.npmjs.com/package/captura)
[![TypeScript types](https://img.shields.io/badge/types-included-blue.svg)](index.d.ts)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cross-platform screen capture for Node.js: native Rust performance, prebuilt
npm binaries, and a fully typed async TypeScript API.

## Features

- **Cross-platform prebuilds** - macOS, Windows, Linux glibc, and Linux musl
  on x64 and arm64.
- **Promise-based API** - every public function returns a `Promise`; capture
  and encoding work is offloaded from the Node.js main thread.
- **TypeScript-first** - generated `.d.ts` declarations with JSDoc ship in the
  npm package.
- **Multiple output formats** - Raw RGBA, PNG, JPEG, WebP, and AVIF.
- **Buffer or Base64 output** - use a Node.js `Buffer` for files and pipelines,
  or an RFC 4648 Base64 string for JSON and data URIs.
- **Normalized monitor metadata** - each monitor exposes physical pixels and
  logical/DIP coordinates.

## Supported Platforms

`captura` requires **Node.js >= 20.3.0** because it targets N-API 9.

| Platform | Architectures | Native packages | Notes |
| --- | --- | --- | --- |
| macOS | x64, arm64 | `captura-darwin-x64`, `captura-darwin-arm64` | Screen Recording permission is required. |
| Windows | x64, arm64 | `captura-win32-x64-msvc`, `captura-win32-arm64-msvc` | No special OS permission is normally required. |
| Linux glibc | x64, arm64 | `captura-linux-x64-gnu`, `captura-linux-arm64-gnu` | Built and release-smoke-tested on Ubuntu 24.04. |
| Linux musl | x64, arm64 | `captura-linux-x64-musl`, `captura-linux-arm64-musl` | Built and smoke-tested on Alpine/musl. |

The root `captura` package loads the matching native package for the current
platform. Keep optional dependencies enabled in your package manager; installing
with `--omit=optional`, `--no-optional`, or an equivalent setting prevents the
native binding from being installed.

## Installation

```bash
npm install captura
```

Use it from ESM/TypeScript:

```ts
import { getMonitors, captureMonitor } from 'captura'
```

Or from CommonJS:

```js
const { getMonitors, captureMonitor } = require('captura')
```

## Quick Start

```ts
import { writeFile } from 'node:fs/promises'
import { getMonitors, captureMonitor } from 'captura'

const monitors = await getMonitors()
const monitor = monitors.find((m) => m.isPrimary) ?? monitors[0]

if (!monitor) {
  throw new Error('No monitors available')
}

const result = await captureMonitor(monitor.id)

await writeFile('screenshot.png', result.screenshot.data)
console.log(
  `Captured ${monitor.friendlyName}: ${result.screenshot.size.width}x${result.screenshot.size.height}`,
)
```

Monitor IDs are assigned by the operating system and may change between
sessions. Discover monitors first, then pass the returned `id` to capture or
lookup functions.

## Recipes

### List Monitors

```ts
const monitors = await getMonitors()

for (const monitor of monitors) {
  const { width, height } = monitor.physical
  console.log(`${monitor.id}: ${monitor.friendlyName} (${width}x${height})`)
}
```

### Get A Specific Monitor

```ts
const [firstMonitor] = await getMonitors()

if (firstMonitor) {
  const monitor = await getMonitorById(firstMonitor.id)
  console.log(monitor.friendlyName)
}
```

`getMonitorById(id)` throws a `[MONITOR_NOT_FOUND]` error when the monitor does
not exist.

### Capture Encoded Images

```ts
import { writeFile } from 'node:fs/promises'

const [monitor] = await getMonitors()

if (monitor) {
  const png = await captureMonitor(monitor.id) // PNG is the default
  await writeFile(`monitor-${monitor.id}.png`, png.screenshot.data)

  const jpg = await captureMonitor(monitor.id, 'Jpeg')
  await writeFile(`monitor-${monitor.id}.jpg`, jpg.screenshot.data)
}
```

### Capture Raw RGBA Pixels

```ts
const [monitor] = await getMonitors()

if (monitor) {
  const raw = await captureMonitor(monitor.id, 'Raw')
  const { width, height } = raw.screenshot.size

  console.log(raw.screenshot.format) // 'Raw'
  console.log(raw.screenshot.data.byteLength === width * height * 4) // true
}
```

Raw output skips image encoding entirely. The buffer layout is RGBA8, 4 bytes
per pixel, row-major, from top-left to bottom-right. Use it when you want to
process pixels yourself with libraries such as `sharp`, `node-canvas`, WebGL,
or a custom encoder.

### Capture All Monitors

```ts
import { writeFile } from 'node:fs/promises'

const results = await captureAllMonitors()

for (const result of results) {
  await writeFile(`monitor-${result.monitor.id}.png`, result.screenshot.data)
}
```

### Capture Base64

```ts
const [monitor] = await getMonitors()

if (monitor) {
  const result = await captureMonitorBase64(monitor.id, 'Png')
  const dataUri = `data:image/png;base64,${result.screenshot.data}`
}

const allAvif = await captureAllMonitorsBase64('Avif')
```

`'Raw'` is not supported by `captureMonitorBase64()` or
`captureAllMonitorsBase64()` because raw pixel data is not self-describing.
Passing `'Raw'` to either Base64 function throws `[INVALID_ARGUMENT]`.

## API

All public functions are async and return promises.

| Function | Returns | Description |
| --- | --- | --- |
| `getMonitors()` | `Promise<Monitor[]>` | Metadata for all connected monitors. |
| `getMonitorById(id)` | `Promise<Monitor>` | Metadata for one monitor; throws `[MONITOR_NOT_FOUND]` if missing. |
| `captureMonitor(id, format?)` | `Promise<CaptureResult>` | Screenshot from one monitor as a `Buffer`. |
| `captureAllMonitors(format?)` | `Promise<CaptureResult[]>` | Screenshots from every monitor as `Buffer`s. |
| `captureMonitorBase64(id, format?)` | `Promise<Base64CaptureResult>` | Screenshot from one monitor as a Base64 string. |
| `captureAllMonitorsBase64(format?)` | `Promise<Base64CaptureResult[]>` | Screenshots from every monitor as Base64 strings. |

### Image Formats

The optional `format` parameter is case-insensitive. Canonical return values are
`'Raw'`, `'Png'`, `'Jpeg'`, `'WebP'`, and `'Avif'`; `'Jpg'` is accepted as an
alias for `'Jpeg'`.

| Value | MIME type | Notes |
| --- | --- | --- |
| `'Raw'` | `application/octet-stream` | Unencoded RGBA8 pixels. Fastest path. Not supported by Base64 functions. |
| `'Png'` | `image/png` | Default. Lossless and pixel-perfect. |
| `'Jpeg'` / `'Jpg'` | `image/jpeg` | Lossy, using default encoder settings. |
| `'WebP'` | `image/webp` | Lossless WebP. |
| `'Avif'` | `image/avif` | Default encoder speed and quality. Usually the slowest built-in encoder. |

### Types

The package ships complete declarations in [index.d.ts](index.d.ts). The core
runtime shapes are:

```ts
type ImageFormat = 'Raw' | 'Png' | 'Jpeg' | 'WebP' | 'Avif'

interface Monitor {
  id: number
  name: string
  friendlyName: string
  physical: Bounds
  logical: Bounds
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

interface Size {
  width: number
  height: number
}

interface CaptureResult {
  monitor: Monitor
  screenshot: Screenshot
}

interface Screenshot {
  size: Size
  format: ImageFormat
  data: Buffer
}

interface Base64CaptureResult {
  monitor: Monitor
  screenshot: Base64Screenshot
}

interface Base64Screenshot {
  size: Size
  format: ImageFormat // Runtime value is never 'Raw'.
  data: string
}
```

### Coordinates

`Monitor.physical` is the pixel-exact monitor geometry. Captured screenshot
dimensions always match `monitor.physical.width` and
`monitor.physical.height` for full-monitor captures.

`Monitor.logical` is the OS/window-manager coordinate space in logical pixels,
DIPs, or CSS points. Use it for UI layout and pointer/window positioning.

`scaleFactor` relates the two coordinate spaces:

```ts
physical = logical * scaleFactor
logical = physical / scaleFactor
```

Monitor `x` and `y` values can be negative when a secondary display is arranged
above or to the left of the primary display.

## Error Handling

All failures are surfaced as JavaScript `Error` objects. The stable captura domain
code is embedded at the start of `err.message` as a `[CODE]` prefix.

The recommended way to recognise captura errors is the dedicated helper
exported from the `captura/errors` subpath. It parses the prefix **and**
validates it against the canonical `CapturaErrorCode` enum exposed by the
native binding, so a positive match is strong evidence the error actually
originated in captura — a third-party error that happens to use a `[FOO]`
prefix will not be misclassified:

```ts
import { captureMonitor, getMonitorById } from 'captura'
import { isCapturaError, CapturaErrorCode } from 'captura/errors'

try {
  await getMonitorById(999999)
} catch (err) {
  if (isCapturaError(err, CapturaErrorCode.MonitorNotFound)) {
    // The monitor id is not available anymore.
  } else if (isCapturaError(err)) {
    // Any other captura-originated error.
  } else {
    throw err
  }
}
```

`getCapturaErrorCode(err)` returns the `[CODE]` string when `err` is a
captura-originated error, or `undefined` otherwise — useful for logging or
switch-style dispatch:

```ts
import { getCapturaErrorCode } from 'captura/errors'

const code = getCapturaErrorCode(err) // e.g. 'MONITOR_NOT_FOUND' or undefined
```

Do not rely on `err.code` for captura domain matching in the current async API.
With napi-rs v3 promise rejections, `err.code` is reserved for the N-API status
code; captura keeps the domain code in the message prefix.

Four categories — `INITIALIZATION_ERROR`, `PERMISSION_DENIED`,
`PLATFORM_NOT_SUPPORTED`, and `TIMEOUT_ERROR` — are **reserved**. They are part
of the stable error enum for forward compatibility but are not emitted by any
current code path; a failed capture today surfaces as `CAPTURE_FAILED`.

| Error code | Description |
| --- | --- |
| `INITIALIZATION_ERROR` | Reserved (not currently emitted). Failure during module or runtime initialization. |
| `MONITOR_NOT_FOUND` | Requested monitor ID does not exist. |
| `CAPTURE_FAILED` | Screenshot operation failed. |
| `PERMISSION_DENIED` | Reserved (not currently emitted). Planned explicit OS screen-capture permission denial; today a denied capture surfaces as `CAPTURE_FAILED`. |
| `PLATFORM_NOT_SUPPORTED` | Reserved (not currently emitted). Feature unavailable on this OS. |
| `ENCODING_ERROR` | Image encoding failure. |
| `INVALID_ARGUMENT` | Invalid parameter, such as an unsupported format string. |
| `INTERNAL_ERROR` | Unexpected internal failure. |
| `TIMEOUT_ERROR` | Reserved (not currently emitted). Operation exceeded time bounds. |
| `RESOURCE_UNAVAILABLE` | OS resource became unavailable. |

## Performance Notes

Capture and encoding work is offloaded from the Node.js main thread. captura does
not impose a global concurrency limit across independent capture calls;
applications that start many high-resolution encoded captures at the same time
should decide their own queueing and backpressure policy based on workload,
latency target, and host CPU budget.

`captureAllMonitors()` and `captureAllMonitorsBase64()` capture monitors
sequentially inside one request to reduce contention in the OS capture
subsystem. They are **fail-fast**: if any monitor capture fails, the whole call
rejects with that error and no partial results are returned.

Use `'Raw'` when you need the fastest path and plan to process or encode pixels
with your own pipeline. Use PNG/JPEG/WebP/AVIF when you need ready-to-write
image files.

## Platform Notes

- **macOS** - Screen Recording permission is required. A denied or otherwise
  failed capture is currently surfaced as `[CAPTURE_FAILED]` with the platform
  error text in the message. A dedicated `[PERMISSION_DENIED]` category is
  reserved for a future release that preflights Screen Recording access.
- **Linux** - X11 and Wayland are supported, subject to the compositor and
  desktop portal environment. Minimal images and containers may need native
  runtime libraries installed before the addon can load.
- **Windows** - No special screen-capture permission is normally required.
- **Headless environments** - Systems without an available display may return
  no monitors or fail capture with a structured error.

## Linux Native Dependencies

captura uses native X11, Wayland, PipeWire, D-Bus, EGL, and GBM libraries on
Linux. Desktop Ubuntu installations usually include many runtime libraries
already, but minimal images and containers often need additional packages.

The published GNU/Linux glibc prebuilt packages are built and release-smoke-
tested on Ubuntu 24.04. Ubuntu 24.04 is the supported prebuilt GNU/Linux
baseline for the current capture stack because the Rust PipeWire bindings
compile against PipeWire/libspa headers newer than Ubuntu 22.04's package set.
Older glibc distributions may require their own build with compatible PipeWire
development headers or an captura version whose dependency graph supports that
distribution.

For Ubuntu 24.04 runtime installations:

```bash
sudo apt-get update
sudo apt-get install -y \
  libxcb1 libxrandr2 libdbus-1-3 \
  libpipewire-0.3-0t64 libwayland-client0 libwayland-server0 \
  libegl1 libgbm1 \
  xdg-desktop-portal
```

Ubuntu 24.04 uses the `libpipewire-0.3-0t64` runtime package name.

If you build captura from source or rebuild the native addon on Ubuntu 24.04,
install development headers too:

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libclang-dev \
  libxcb1-dev libxrandr-dev libdbus-1-dev \
  libpipewire-0.3-dev libwayland-dev libegl-dev libgbm-dev
```

### Unsupported Linux Targets

If the prebuilt GNU or musl package cannot load on your distribution, build the
native addon on that target system so it links against that system's libc and
desktop capture libraries. You need Node.js 20.3.0 or newer, Rust, and
PipeWire/libspa development headers compatible with the current capture
dependencies.

The published npm package is prebuilt-only and does not include Rust source, so
source builds start from the repository tag that matches the package version you
want to run:

```bash
git clone https://github.com/sandyfzu/captura.git
cd captura
git checkout v1.0.0 # replace with the captura version you installed
npm ci
npm run build
```

Then, from the application that has `captura` installed, point the generated
loader at the locally built `.node` file:

```bash
export NAPI_RS_NATIVE_LIBRARY_PATH="/absolute/path/to/captura.linux-x64-gnu.node"
node -e "const captura = require('captura'); console.log(Object.keys(captura))"
```

Use the `.node` file produced for your actual platform and architecture, such
as `captura.linux-arm64-gnu.node` on Linux ARM64 glibc or
`captura.linux-x64-musl.node` on Alpine x64.

## Development

The published package supports Node.js 20.3.0 or newer. Repository development
targets Node.js 24 or newer, declared via the `devEngines` field in
`package.json` and matched by CI. The minimum the dev tooling will tolerate is
Node.js 22.22.1 (required by lint-staged 17).

```bash
npm ci
npm run build
npm test
npm run typecheck
cargo test --workspace --locked
```

Maintainers preparing a release should follow [RELEASE.md](RELEASE.md). The
release workflow builds all eight native targets, generates the platform npm
packages, validates tarballs, smoke-tests installs, and publishes with npm
Trusted Publishing and signed build provenance.

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

[MIT](LICENSE)
