# Release Readiness

This document is the maintainer checklist for publishing xshot as a stable npm
native module. It reflects the current package layout, the generated NAPI-RS
loader, and the audit performed before the first stable release.

## Current Release Status

xshot is not ready for a stable npm release until the native package publishing
flow is verified end to end. The Rust and JavaScript validation gates pass
locally, but a root-package dry run currently packs only the JavaScript loader,
TypeScript declarations, README, license, changelog, and package metadata. The
generated loader expects either a local `.node` file or a platform package such
as `xshot-darwin-arm64`, `xshot-linux-x64-gnu`, or `xshot-win32-x64-msvc`.

The stable release must therefore prove that the published root package declares
the expected platform packages and that each platform package contains the
matching native binary.

## P0 Stable Release Blockers

### Native Package Distribution

Use the NAPI-RS platform-package model. The root npm package should depend on
platform-specific native packages through `optionalDependencies`, and each
native package should restrict itself with the appropriate npm `os`, `cpu`, and
where applicable `libc` metadata.

Required actions:

1. Build every configured NAPI target in CI.
2. Download all native build artifacts into an `artifacts/` directory.
3. Run `npm run artifacts` so `napi artifacts` copies each `.node` file into
   its generated package under `npm/`.
4. Run `napi prepublish -t npm --dry-run` and inspect the root package and
   generated platform package metadata.
5. Run `napi prepublish -t npm` during the real publish flow only after the dry
   run is correct.
6. Pack the root package and every generated package with `npm pack --dry-run
   --json` first, then with `npm pack` in a temporary release directory.
7. Install the packed root tarball in a clean temporary project and verify that
   `require('xshot')` or `import('xshot')` loads the native binding on that
   platform.

Do not publish a stable release if the root tarball has no matching native
package path for the current platform.

### Release Workflow

Create a dedicated release workflow before the stable tag. The workflow should
run after the CI gate and should not rely on artifacts left in a developer
checkout.

Required actions:

1. Build or download all platform artifacts for the exact commit being tagged.
2. Run the package generation steps from this document.
3. Verify packed contents for root and platform packages.
4. Run a clean-install smoke test from tarballs.
5. Publish platform packages and the root package only after all tarball checks
   pass.

## P1 Release Readiness Items

### Target Matrix Consistency

Keep these lists synchronized:

1. `package.json` `napi.targets`.
2. CI build matrix.
3. `deny.toml` graph targets.
4. The generated platform packages under `npm/`.
5. README supported-platform claims.

The audit found that `aarch64-unknown-linux-musl` is advertised as a build target
but should also be represented in the cargo-deny graph target list before the
first stable release.

### Package Metadata

The npm package should include useful metadata for users and tooling:
`repository`, `bugs`, `homepage`, `keywords`, `license`, `main`, `types`, and an
accurate `engines.node` range. The root package metadata must stay synchronized
with generated platform packages during `napi prepublish`.

### Platform Smoke Tests

CI should keep fast PR checks, but the stable release workflow should smoke-test
each published target from the exact tarballs that will be published. At minimum,
verify that the package loads, exports the six public async functions, and can
return a structured error for an invalid format without requiring a physical
display.

## Verification Checklist

Run these checks before creating a release candidate:

```bash
npm ci
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
npm run typecheck
npm test
npm run build
npm audit --json
cargo deny check
npm pack --dry-run --json
```

Run these checks in the release workflow after all platform artifacts are
available:

```bash
npm run artifacts
napi prepublish -t npm --dry-run
npm pack --dry-run --json
```

Then pack root and generated platform packages into a temporary directory,
install from the root tarball in a clean project, and run a load smoke test:

```bash
node -e "const x = require('xshot'); console.log(Object.keys(x).sort())"
node -e "const x = require('xshot'); x.captureAllMonitors('bad').catch(e => console.log(e.message))"
```

## Trusted References

- [NAPI-RS release guidance](https://napi.rs/docs/deep-dive/release)
- [NAPI-RS artifact packaging](https://napi.rs/docs/cli/artifacts)
- [NAPI-RS prepublish command](https://napi.rs/docs/cli/pre-publish)
- [npm package manifest fields](https://docs.npmjs.com/cli/v10/configuring-npm/package-json)
- [npm pack dry-run behavior](https://docs.npmjs.com/cli/v10/commands/npm-pack)
- [Tokio `spawn_blocking`](https://docs.rs/tokio/1.51.1/tokio/task/fn.spawn_blocking.html)
- [Node-API ABI and error handling](https://nodejs.org/api/n-api.html)
