# AGENTS.md — captura

## Project Identity

**captura** is a production-grade Node.js native module implemented in Rust. It is a high-level, ergonomic, and safe wrapper around the `xcap` crate, using `napi-rs` as the binding layer to expose cross-platform screen capture capabilities to Node.js and TypeScript consumers.

The module is designed to be published as an npm package with full TypeScript type definitions, platform-specific binary distribution, and a promise-based async API.

---

## Target Platforms

- macOS (x64 and arm64)
- Linux (X11 and Wayland)
- Windows (x64 and arm64)

All three platforms are first-class citizens. The public API must behave consistently across platforms. Platform-specific differences in data (e.g., DPI scaling, monitor naming, coordinate systems) must be normalized at the Rust layer before reaching JavaScript.

---

## Architecture

The codebase is a workspace organized into four clearly separated layers. Do not mix concerns across layers. Each layer is a separate crate in the workspace.

### Core Layer (Rust / xcap abstraction)

- Pure Rust logic that interfaces with the `xcap` crate.
- No Node.js or NAPI-rs types in this layer.
- Wraps `xcap::Monitor`, `xcap::Window`, and related types.
- Handles all platform-specific normalization (DPI, coordinates, monitor metadata differences across macOS/Linux/Windows).
- Returns domain types, not raw `xcap` types.

### Domain Layer

- Defines domain models (`Monitor`, `Screenshot`, `MonitorInfo`, etc.).
- Contains business logic independent of any transport or binding mechanism.
- Models are plain Rust structs; they do not derive or implement NAPI traits.

### Interop Layer (NAPI-rs bindings)

- The **only** layer that imports `napi` and `napi_derive`.
- Responsible for exposing public functions to Node.js via `#[napi]` macros.
- Converts domain types into NAPI-compatible types for serialization.
- Converts Rust errors into JavaScript `Error` objects with structured codes and messages.
- All exposed functions are `async` and return `Promise` to JavaScript.

### Utility Layer

- Shared helpers: error conversion, image encoding, buffer manipulation, cross-platform normalization logic.
- Used by multiple layers; must not depend on NAPI types.

---

## Rust Coding Standards

### General

- Write idiomatic Rust. Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- All code must be **Clippy-clean** with no warnings.
- Use `Result<T, E>` for all fallible operations. Propagate errors with `?`.
- **Never use `unwrap()` or `expect()` in production code paths.** These are only acceptable in tests.
- Use `thiserror` for defining custom error types.
- Use `cfg` attributes for platform-specific code; isolate platform-specific implementations into separate modules.

### Functions

- Functions must be small, focused, and adhere to single-responsibility.
- Decompose complex logic into reusable internal functions.
- No duplicated logic; extract shared behavior into reusable internal functions. Use the utility layer for logic that is shared across multiple layers.

### Safety

- No `unsafe` code unless absolutely required and thoroughly documented with `// SAFETY:` comments explaining the invariant.
- **Panics must never cross the FFI boundary.** Use `std::panic::catch_unwind` at the interop layer boundary if calling code that could theoretically panic.
- No undefined behavior. Rust's ownership and borrowing rules must be respected without workarounds.
- No resource leaks: buffers, file handles, and OS handles must be properly released.

### Logging

- Use the `log` crate for structured logging when useful.
- Do not log sensitive information.

---

## Error Handling

### Critical Constraint

**Panics must never propagate to JavaScript.** Every panic must be caught at the interop boundary and converted into a structured error.

### Error Type Design

Define a unified error enum (e.g., `CapturaError`) using `thiserror`. Every error variant must include:

- A clear, human-readable message.
- An error category/code (string identifier for programmatic matching on the JS side).
- Contextual metadata when applicable (e.g., monitor ID, platform name).

### Error Categories

The error enum should cover at least these categories:

| Category | Description |
| --- | --- |
| `INITIALIZATION_ERROR` | Failure during module or runtime initialization |
| `MONITOR_NOT_FOUND` | Requested monitor ID does not exist |
| `CAPTURE_FAILED` | Screenshot or capture operation failed |
| `PERMISSION_DENIED` | OS denied screen capture permission (common on macOS) |
| `PLATFORM_NOT_SUPPORTED` | Feature unavailable on the current OS |
| `ENCODING_ERROR` | Image encoding/conversion failure |
| `INVALID_ARGUMENT` | Invalid parameter passed by the caller |
| `INTERNAL_ERROR` | Unexpected internal failure (catch-all) |
| `TIMEOUT_ERROR` | Operation exceeded expected time bounds |
| `RESOURCE_UNAVAILABLE` | OS resource (monitor, window) became unavailable |

Additional categories should be added when they make semantic sense for the domain.

### JavaScript Representation

- Errors are surfaced as standard JavaScript `Error` objects.
- The current stable JavaScript contract embeds the captura domain code in the
  beginning of `err.message` as `[CODE] description`.
- Do not document or test custom `err.code` for async exports unless the
  implementation has been changed and verified. With napi-rs v3 async promise
  rejections, `err.code` is reserved for the NAPI status code rather than the
  captura domain category.
- Prefer structured error categories over plain messages. If a future napi-rs
  version supports domain-specific promise rejection codes, update the Rust
  interop tests, TypeScript docs, README, and this file together.

### Public Error Contract

The stable JavaScript contracts are:

1. **Wire-level**: `err.message` always starts with `[CODE]` where `CODE` is
   one of the canonical captura categories. This is what crosses the FFI
   boundary and what every test pins.
2. **Helper-level (recommended for consumers)**: the `captura/errors` subpath
   exports `isCapturaError`, `getCapturaErrorCode`, and `CapturaErrorCode`.
   The helpers parse the `[CODE]` prefix **and** validate it against the
   canonical `CapturaErrorCode` enum exported from the native binding, so a
   third-party error that happens to use a `[FOO]` prefix is not
   misclassified.

Do not document custom `err.code` matching unless the interop layer and
integration tests are changed to prove that custom domain codes survive async
promise rejections. With napi-rs v3, `err.code` is reserved for the N-API
status string (typically `"GenericFailure"`).

Required actions:

1. Keep README examples aligned with both the helper-based and message-prefix
   contracts. The helper is the recommended path; the prefix is the
   underlying contract that the helper relies on.
2. Keep integration tests checking `[INVALID_ARGUMENT]`, `[MONITOR_NOT_FOUND]`,
   and any future public error category through `isCapturaError(err, code)`
   plus at least one direct `err.message.startsWith('[CODE]')` regression
   guard so the wire format cannot drift silently.
3. The `CapturaErrorCode` JS enum is generated from the Rust
   `JsCapturaErrorCode` enum in `crates/interop/src/types.rs` via
   `#[napi(string_enum = "UPPER_SNAKE")]` so the wire codes, the Rust
   variants, and the JS enum values stay in lockstep. Adding a domain error
   category requires updating: (a) `captura_domain::CapturaErrorCode`,
   (b) `JsCapturaErrorCode` plus its `From<CapturaErrorCode>` impl,
   (c) the README error table, and (d) the integration tests.
4. Treat a future switch to custom `err.code` as a breaking public API change
   unless both matching forms are supported for a full major cycle.
5. Keep errors.d.ts file aligned with the public error contract and documented categories, same for manual lists of error categories in README and tests.

---

## Concurrency & Async Model

- Use **tokio** as the async runtime. NAPI-rs has built-in tokio integration — `async fn` decorated with `#[napi]` automatically runs on the tokio runtime and returns a JavaScript `Promise`.
- All I/O, capture operations, and potentially blocking work must be offloaded to async tasks.
- **Never block the Node.js main thread.**
- Use `tokio::task::spawn_blocking` for CPU-bound or synchronous xcap calls that cannot be made async.
- Do not spawn unnecessary tasks; keep the concurrency model simple and predictable.
- Do not add a global concurrency limit for all independent capture/encode calls
  unless there is measured evidence and an explicit API design. Application-level
  queueing and backpressure are consumer policy decisions; captura should document
  the cost of concurrent high-resolution encodes and keep `captureAllMonitors*`
  internally conservative.

---

## Cross-Platform Requirements

### API Consistency

- The public API surface must be identical across macOS, Linux, and Windows.
- Monitor metadata (resolution, position, scale factor, DPI, name) must be normalized to a consistent structure regardless of OS.
- Platform-specific differences in what `xcap` returns (e.g., macOS uses different DPI semantics than Windows, Linux X11 vs Wayland differences) must be handled transparently in the core layer.

### Conditional Compilation

- Use `#[cfg(target_os = "...")]` to isolate platform-specific code.
- Keep platform-specific modules separate (e.g., `platform/macos.rs`, `platform/linux.rs`, `platform/windows.rs`).
- The interop layer must never contain `cfg` attributes — all platform branching must be resolved in the core or utility layers.

### Graceful Degradation

- If a feature is unsupported on a platform (e.g., Wayland has limited screen capture in certain scenarios), return a clear `PLATFORM_NOT_SUPPORTED` error instead of panicking or returning garbage data.

---

## Node.js / TypeScript API Design

### Goals

- Intuitive, minimal-friction API.
- Fully typed with TypeScript definitions (auto-generated by `napi-rs`).
- Promise-based — every function returns a `Promise`.
- Predictable: no ambiguous return values, no implicit behavior.

### Naming Conventions

- Use camelCase for all public function names (NAPI-rs converts Rust snake_case automatically).
- Use clear, descriptive names: `getMonitors`, `getMonitorById`, `captureMonitor`, `captureAllMonitors`, `captureMonitorBase64`, `captureAllMonitorsBase64`.
- Avoid abbreviations unless universally understood.

### API Shape

```ts
// Monitor management
const monitors: Monitor[] = await getMonitors()
const monitor: Monitor = await getMonitorById(id) // throws MONITOR_NOT_FOUND if id does not exist

// Screen capture — Buffer output (PNG by default, optional format parameter)
const result: CaptureResult = await captureMonitor(id)            // PNG
const jpgResult: CaptureResult = await captureMonitor(id, 'Jpeg') // JPEG
const results: CaptureResult[] = await captureAllMonitors()       // all monitors, PNG
const avifResults: CaptureResult[] = await captureAllMonitors('Avif')

// Screen capture — Raw RGBA pixels (fastest, no encoding overhead)
const raw: CaptureResult = await captureMonitor(id, 'Raw')
// raw.screenshot.data is a Buffer of width × height × 4 unencoded RGBA bytes

// Screen capture — Base64 string output (same format selection, except Raw)
const b64Result: Base64CaptureResult = await captureMonitorBase64(id)       // PNG
const b64Jpg: Base64CaptureResult = await captureMonitorBase64(id, 'Jpeg')  // JPEG
const b64All: Base64CaptureResult[] = await captureAllMonitorsBase64()      // all monitors
// Note: passing 'Raw' to Base64 functions throws INVALID_ARGUMENT
```

### Data Structures

- Return strongly typed interfaces/objects.
- No ambiguous return values — prefer throwing a structured error over returning `null` or `undefined`. Only return `null` when absence is a valid, expected state (e.g., an optional field with no value on a given platform).
- `getMonitorById` must throw a `MONITOR_NOT_FOUND` error when the ID does not exist — it must never return `null`.
- `CaptureResult` pairs a monitor's metadata with its captured image. `Base64CaptureResult` is identical but carries a Base64 string instead of a `Buffer`:

```ts
interface CaptureResult {
  monitor: Monitor
  screenshot: Screenshot
}

interface Screenshot {
  size: Size
  format: ImageFormat  // 'Raw' | 'Png' | 'Jpeg' | 'WebP' | 'Avif'
  data: Buffer         // Raw RGBA8 pixels (format === 'Raw') or encoded image bytes
}

interface Base64CaptureResult {
  monitor: Monitor
  screenshot: Base64Screenshot
}

interface Base64Screenshot {
  size: Size
  format: ImageFormat  // 'Png' | 'Jpeg' | 'WebP' | 'Avif' (Raw not allowed)
  data: string         // RFC 4648 Base64-encoded image
}
```

- Monitor metadata exposes both physical and logical geometry via nested `Bounds` objects:

```ts
interface Monitor {
  id: number
  name: string
  friendlyName: string
  physical: Bounds       // Geometry in physical pixels (matches screenshot dimensions)
  logical: Bounds        // Geometry in logical / DIP units
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
```

---

## Memory & Data Handling

- Use `Buffer` (via NAPI-rs `Buffer` type) for returning image data to JavaScript, or `string` for Base64-encoded output.
- **Two data paths exist** — the Raw path and the encoded path. Both return a `Buffer`, but with different contents:
  - **Raw** (`'Raw'` format): the `Buffer` contains the RGBA8 pixel data with no compression and no encoding applied. This is the **fastest capture path** because it skips all image processing. The buffer layout is 4 bytes per pixel (R, G, B, A), row-major, top-left to bottom-right, and its length is always `width × height × 4`. Use Raw when you plan to process pixels yourself (e.g. feed into `sharp`, draw on a canvas, upload as a WebGL texture, or re-encode with custom quality settings).
  - **Encoded** (PNG, JPEG, WebP, AVIF): the `Buffer` contains a complete image file (e.g. a valid `.png`). It can be written to disk, served over HTTP, or passed to any image library without additional processing. Use an encoded format when you need a ready-to-use image file.
- **Base64 variants** (`captureMonitorBase64`, `captureAllMonitorsBase64`) return the encoded image data as an RFC 4648 Base64 string instead of a `Buffer`. Base64 encoding is performed on the Rust side using the `base64` crate before crossing the FFI boundary. **Raw is not supported for Base64** — passing `'Raw'` to a Base64 function returns an `INVALID_ARGUMENT` error because raw pixel data is not self-describing and has no meaningful MIME type for data URIs.
- Encoding is performed on the Rust side using the `image` crate (a direct dependency, also a transitive dependency via `xcap`) before transferring ownership to JavaScript.
- Supported encoding formats (PNG, JPEG, WebP, AVIF) are selected via an optional parameter (PNG default) and handled entirely in the utility layer. The interop layer passes the option through; it does not contain encoding logic. All formats use default encoder settings — WebP is lossless only.
- **Raw bypasses the utility encoding layer entirely.** In the core layer, `RgbaImage::into_raw()` moves the underlying `Vec<u8>` to the `Screenshot` struct without additional copies or allocations. The interop layer then wraps this `Vec<u8>` into a NAPI `Buffer`, which transfers ownership to the V8 garbage collector — again without copying. Note that the upstream capture library normalises the OS-native pixel format (e.g. BGRA, various bit depths) to RGBA8 before captura receives the buffer.
- For encoded formats, avoid unnecessary memory copies. Encode once on the Rust side and transfer ownership to JavaScript.
- Large image buffers must not be cloned unnecessarily.
- Be mindful of buffer sizes, especially for high-DPI monitors (source RGBA is width × height × 4 bytes — e.g. a 3840×2160 display produces ~33 MB of raw pixel data before encoding).

---

## Testing Strategy

### Rust Unit Tests

- Write unit tests for:
  - Core logic (monitor listing, metadata normalization, capture orchestration).
  - Error handling (every error variant is constructible and renders a meaningful message).
  - Edge cases (no monitors available, invalid IDs, zero-size regions).
  - Cross-platform normalization logic (mock platform-specific inputs where feasible).
  - Utility functions (encoding, conversion, buffer helpers).

### Node.js Integration Tests

- Write integration tests that exercise the compiled `.node` binary:
  - Validate that all exported functions exist and return the expected types.
  - Validate async behavior (functions return Promises that resolve correctly).
  - Validate error propagation through the public `[CODE]` message-prefix
    contract for async exports.
  - Validate returned data structures match the TypeScript interfaces.
- Keep `__tests__/integration.mts` and `__tests__/smoke-report.mts`
  aligned with the public JavaScript API. Whenever a new exported function,
  parameter, return field, image format, Base64 variant, error category, or
  behavior-changing default is added, update both the automated integration
  tests and the smoke report runner in the same change.
- The smoke report runner is observation-oriented rather than
  assertion-oriented. It must exercise every public function and supported
  variant, save monitor/capture/error/timing artifacts under the ignored
  `captura-smoke-reports/` directory, and generate a self-contained HTML report
  that relates monitor metadata to the screenshots produced by each API.

### Cross-Platform Validation

- Tests must be designed to pass on macOS, Linux, and Windows.
- Use CI (GitHub Actions with matrix builds) to validate all platforms.
- Tests that depend on a physical display should be skippable in headless CI environments with clear skip messages rather than failures.

---

## Dependency Policy

- Use the latest stable versions of all core dependencies.
- All dependencies must be actively maintained and compatible with each other.
- All dependencies must be declared in the main `Cargo.toml` workspace file and pulled into individual crates (`Cargo.toml`) from the main one as needed.
- Core Rust dependencies:
  - `xcap` — screen capture abstraction
  - `napi` and `napi-derive` — Node.js binding framework
  - `napi-build` — build script helper
  - `tokio` — async runtime (with the `async` feature in napi)
  - `thiserror` — ergonomic error types
  - `image` — image encoding (also a transitive dependency via `xcap`; added as a direct dependency because the utility layer uses encoder types — `PngEncoder`, `JpegEncoder`, `WebPEncoder`, `AvifEncoder` — and the `ImageEncoder` trait that `xcap` does not re-export)
  - `base64` — RFC 4648 Base64 encoding for the Base64 screenshot API
- Node.js tooling:
  - `@napi-rs/cli` — build and publish toolchain
- Do not add dependencies unless they provide clear, justified value. Prefer standard library solutions when they exist.
- All dependencies and their versions must be taken into account when checking documentation to ensure compatibility.

---

## Build & Packaging

- The crate type must be `cdylib` for dynamic library output.
- Include a `build.rs` that calls `napi_build::setup()`.
- Use `@napi-rs/cli` for building, creating npm directories, and publishing platform-specific packages.
- Follow the NAPI-rs distribution model: one root npm package with `optionalDependencies` pointing to platform-specific binary packages.
- Use npm Trusted Publishing through GitHub Actions OIDC for release publishes;
  do not add long-lived npm publish tokens unless Trusted Publishing is
  unavailable and the release process is explicitly redesigned. For first-ever
  publishes of new npm package names, use only the explicit one-time token
  bootstrap path documented in `RELEASE.md`; never make token fallback implicit.
- TypeScript type definitions (`.d.ts`) and loader files (`index.js`) that are
  auto-generated by NAPI-rs must not be hand-written or manually patched.
- If generated loader package names are wrong, fix the NAPI-rs configuration and
  regenerate outputs with the CLI instead of editing generated files.
- Take into account the engine and N-API version compatibility when building and publishing (Check package.json and Cargo.toml files).
- Before publishing a stable release, run the release checklist in `RELEASE.md`:
  collect CI artifacts, run `napi artifacts`, run `napi prepublish -t npm`, pack
  the root and platform packages, and smoke-test installation from the produced
  tarballs.

---

## Production Quality Requirements

- **No undefined behavior** — ever.
- **Defensive programming**: validate inputs at system boundaries (function entry points in the interop layer). Do not over-validate internal code.
- **Graceful degradation**: when the OS or environment is in an unexpected state, return a structured error rather than crashing.
- **Memory safety**: enforced by Rust's type system. Do not circumvent it.
- **No resource leaks**: all OS handles, buffers, and allocated memory must be properly released.
- **Stable under load**: multiple concurrent capture calls must not corrupt state or crash.

---

## Extensibility

The architecture must support future additions without requiring structural changes.

### Implemented features

- **Raw capture** — Passing `'Raw'` as the format returns the RGBA8 pixel buffer with no encoding or compression. This is the fastest capture path and the recommended choice when pixels will be processed, re-encoded with custom settings, or fed into libraries like `sharp` or a `<canvas>`. Raw bypasses the utility encoding layer entirely — `RgbaImage::into_raw()` moves the `Vec<u8>` through to JavaScript without additional copies. Raw is not supported for Base64 functions.
- **Multi-format encoding** — PNG (default), JPEG, WebP (lossless only), and AVIF are supported via an optional `format` parameter on every capture function. All use default encoder settings. Additional formats or fine-grained configuration can be added by extending the utility layer.
- **Base64 encoding** — `captureMonitorBase64` and `captureAllMonitorsBase64` return screenshots as RFC 4648 Base64 strings instead of Buffers. Base64 encoding is performed on the Rust side using the `base64` crate.

### Future extension points

- **Window capture** — `xcap` already provides `Window::all()` and `window.capture_image()`.
- **Region capture** — `xcap` supports `monitor.capture_region(x, y, width, height)`.
- **Video recording** — `xcap` provides `monitor.video_recorder()` (marked WIP upstream).
- **Streaming APIs** — future streaming support can be added via NAPI-rs `ThreadsafeFunction`.
- **Save to disk** — optional file output can be added as a convenience API.
- **Additional metadata** — expose more monitor properties as needed (color depth, HDR support, etc.).

When extending, add new modules rather than modifying existing ones. Follow the existing layer separation.

---

### Documentation

- All functions (especially public ones) must have doc comments that describe their behavior, parameters, return values, and possible errors.
- Document the error enum with clear descriptions of each variant and when it is used.
- Document any `unsafe` code with detailed safety invariants.
- The README.md should provide usage examples, installation instructions, and a high-level overview of the API.
- Internal documentation should explain the architecture and design decisions where non-obvious.
- All documentation must be clear, concise, and free of jargon.
- Document platform-specific behavior and differences clearly.
- Use examples in documentation to illustrate common usage patterns and edge cases.
- Documentation must be kept up-to-date with code changes. Outdated documentation is worse than no documentation.
- Document the expected behavior in error scenarios (e.g., what happens if permissions are denied on macOS? What error is thrown if an invalid monitor ID is used?).
- Documentation must be ready to generate API reference docs using `cargo doc` without additional manual editing.
- Some implementation details like xcap, napi-rs, image crate, etc specifics must be omitted from public documentation but should be covered in internal docs for maintainers.

---

## What to Avoid

- **Do not add `unwrap()` or `expect()` in any non-test code.**
- **Do not introduce `unsafe` without documented justification and a `// SAFETY:` comment.**
- **Do not put NAPI types in the core or domain layer.**
- **Do not put `cfg` attributes in the interop layer.**
- **Do not block the Node.js event loop.**
- **Do not return raw `xcap` types to JavaScript** — always map through domain types.
- **Do not duplicate logic** — extract into reusable helpers.
- **Do not add features or abstractions beyond what is needed for the current task.**
- **Do not hand-write `.d.ts` files** — NAPI-rs generates them.
- **Do not hand-edit generated NAPI loader files** (`index.js` or generated
  interop loader/type files). Regenerate through NAPI-rs instead.
- **Do not swallow errors silently** — every error must be surfaced or explicitly logged.
- **Do not use `println!` or `eprintln!` for logging** — use the `log` crate.

---

## This file (AGENTS.md)

- This file is a living document that captures the architectural vision, coding standards, and design principles for the captura project.
- It is intended to guide contributors and maintainers in writing high-quality, consistent code that adheres to the project's goals and constraints.
- This file should be updated whenever there are significant changes to the architecture, coding standards, or design principles. It serves as the single source of truth for how the project should be structured and developed.
