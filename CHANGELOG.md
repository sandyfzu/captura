# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Corrected public documentation for async error matching: xshot domain error
  codes are documented as `[CODE]` message prefixes instead of a custom
  `err.code` property.
- Added maintainer release-readiness documentation for NAPI-RS platform package
  generation, packed-tarball verification, and first-stable release gates.
- Added a dedicated release workflow that builds all native targets, generates
  NAPI-RS platform packages, validates tarballs, smoke-tests installs, and
  publishes platform packages before the root package.
- Kept GNU/Linux release builds and installed-package smoke coverage on Ubuntu
  24.04 because the current pipewire-rs/libspa bindings require PipeWire headers
  newer than Ubuntu 22.04's package set, and documented source-build guidance
  for older or unsupported Linux targets.
- Expanded CI to build and test every advertised native package target on
  matching GitHub-hosted runner architectures, using same-arch Alpine Docker for
  musl targets instead of QEMU or Linux cross-compilation.
- Hardened CI artifact handling, shell selection, npm cache configuration, and
  manual dispatch support for the native build/test matrix.
- Switched release publishing guidance and automation to npm Trusted Publishing
  with GitHub Actions OIDC instead of long-lived npm publish tokens, and added
  explicit release dry-run instructions.
- Added an explicit one-time token-bootstrap publish path for the first release
  of brand-new npm package names before npm Trusted Publishing can be configured.
- Clarified that high-concurrency encode scheduling is an application-level
  policy decision; xshot offloads blocking work but does not impose a global
  concurrency limit across independent calls.

### Fixed

- Clarified macOS permission-denied behavior so the README no longer promises
  `PERMISSION_DENIED` when the upstream OS/capture error is not distinguishable.
- Installed the Wayland server runtime package in Linux GNU CI and release
  smoke jobs so the native binding can resolve `libwayland-server.so.0`.
- Generated NAPI-RS package entrypoints in release source verification before
  TypeScript typechecking so clean checkouts can resolve the public API module.
- Skipped NAPI-RS optional package publishing during release package metadata
  finalization so unauthenticated packaging jobs do not call `npm publish`.
- Pinned Windows x64 CI and release jobs to GitHub's `windows-2025-vs2026`
  runner image ahead of the `windows-2025` redirect deadline.
- Scoped Rust workflow caches by runner image and target to prevent cached
  build-script binaries from crossing Ubuntu glibc baselines.
- Added the missing `aarch64-unknown-linux-musl` target to cargo-deny's graph
  target list so dependency checks match the advertised NAPI target matrix.
- Updated the transitive `bitstream-io` lockfile entry to remove the yanked
  `core2` crate from the dependency graph.

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
