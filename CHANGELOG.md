# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-04-10

Initial release of xshot — a cross-platform screen capture native module for
Node.js, built with Rust.

### Added

- **Monitor discovery** — `getMonitors()` returns metadata for all connected
  monitors; `getMonitorById(id)` returns a single monitor or throws
  `MONITOR_NOT_FOUND`.
- **Screen capture (Buffer)** — `captureMonitor(id, format?)` and
  `captureAllMonitors(format?)` return screenshots as Node.js `Buffer` objects.
- **Screen capture (Base64)** — `captureMonitorBase64(id, format?)` and
  `captureAllMonitorsBase64(format?)` return screenshots as RFC 4648 Base64
  strings.
- **Raw format** — pass `'Raw'` to receive the unencoded RGBA8 pixel buffer
  with no image encoding or compression — the fastest capture path.
- **Multi-format encoding** — PNG (default), JPEG, WebP (lossless), and AVIF,
  selectable via an optional `format` parameter on every capture function.
  Format parsing is case-insensitive and accepts `'Jpg'` as an alias for
  `'Jpeg'`.
- **Physical and logical coordinates** — every `Monitor` exposes both
  `physical` (pixel-exact) and `logical` (DIP / CSS-point) `Bounds` objects,
  normalised across macOS, Linux, and Windows.
- **Cross-platform normalisation** — macOS `CGDisplayBounds` logical points and
  Linux XRandR logical values are converted to physical pixels so that
  `width × height` always matches the captured image dimensions.
- **Structured error handling** — 10 error categories (`INITIALIZATION_ERROR`,
  `MONITOR_NOT_FOUND`, `CAPTURE_FAILED`, `PERMISSION_DENIED`,
  `PLATFORM_NOT_SUPPORTED`, `ENCODING_ERROR`, `INVALID_ARGUMENT`,
  `INTERNAL_ERROR`, `TIMEOUT_ERROR`, `RESOURCE_UNAVAILABLE`) surfaced as
  JavaScript `Error` objects with `[CODE]` prefixed messages.
- **Async API** — every public function returns a `Promise`. Blocking OS calls
  are offloaded to background threads and never stall the Node.js event loop.
- **TypeScript definitions** — auto-generated `.d.ts` with rich JSDoc
  documentation, including platform-specific tables for monitor name fields.
- **8 build targets** — macOS x64/arm64, Windows x64/arm64, Linux GNU
  x64/arm64, Linux musl x64/arm64.
- **CI pipeline** — GitHub Actions workflow with Rust quality checks (fmt,
  clippy, tests, docs) on 3 platforms, native module builds for all 8 targets,
  JS integration tests, and `cargo-deny` for license and advisory auditing.
- **Git hooks** — Husky pre-commit (rustfmt via lint-staged) and pre-push
  (full 6-step CI-equivalent validation).
- **83 Rust unit tests** across 4 crates and **34 Node.js integration tests**
  covering module exports, error handling, live capture, format coverage, and
  Raw buffer size validation.

[unreleased]: https://github.com/sandyfzu/xshot/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/sandyfzu/xshot/releases/tag/v0.9.0
