# Release Runbook

xshot is a native Node.js module. The root npm package requires a matching
platform package containing the correct `.node` binary for each target. A broken
or incomplete publish can leave consumers unable to load the module. Follow this
runbook for every stable release.

The release workflow lives in `.github/workflows/release.yml`. It builds all
eight configured native targets, generates platform npm packages, validates and
smoke-tests the packed tarballs, and publishes only after a manual dispatch with
`publish: true`.

---

## Required Repository Configuration

Configure these settings once before the first publish. For normal releases after
package names already exist on npm, only the `npm-production` Trusted Publishing
environment is needed.

### npm Package Names

Ensure the npm account `sandyfzu` owns all nine package names before publishing:

| Package | Purpose |
| ------- | ------- |
| `xshot` | Root package — installed by consumers |
| `xshot-darwin-x64` | macOS Intel binary |
| `xshot-darwin-arm64` | macOS Apple Silicon binary |
| `xshot-win32-x64-msvc` | Windows x64 binary |
| `xshot-win32-arm64-msvc` | Windows ARM64 binary |
| `xshot-linux-x64-gnu` | Linux x64 glibc binary |
| `xshot-linux-arm64-gnu` | Linux ARM64 glibc binary |
| `xshot-linux-x64-musl` | Linux x64 musl (Alpine) binary |
| `xshot-linux-arm64-musl` | Linux ARM64 musl (Alpine) binary |

The GNU/Linux glibc packages are built and smoke-tested on Ubuntu 24.04. This is
the current prebuilt baseline because xcap's current pipewire-rs/libspa
dependency stack compiles against PipeWire/libspa headers newer than Ubuntu
22.04's package set. These packages depend on glibc and the native
X11/Wayland/PipeWire libraries used by the capture stack, so the Rust target's
glibc floor alone is not a full older-distribution compatibility promise. Older
distributions require explicit validation, compatible PipeWire development
headers, or a source build on the target system. Alpine/musl packages are built
and smoke-tested separately inside Alpine.

### GitHub Environment: `npm-production`

This environment gates all normal Trusted Publishing releases.

1. On GitHub, go to **Settings → Environments → New environment**.
2. Name it `npm-production`.
3. Click **Configure environment** to open the environment settings page.
4. Under **Deployment protection rules**, select **Required reviewers**, add at
   least one reviewer, then click **Save protection rules**.

   > **Note:** Required reviewers is only available for **public repositories** on
   > GitHub Free/Pro. For private repositories it requires GitHub Team or
   > Enterprise. If the option is not visible, either make the repository public
   > or upgrade the plan. Without this gate, any workflow run can publish to npm
   > without a human approval step.

5. Still under **Deployment protection rules**, add a **Deployment branches and
   tags** rule: select **Tag**, enter the pattern `v*`, then click **Add rule**.

### npm Trusted Publishing

Trusted Publishing lets npm verify publish requests via GitHub Actions OIDC — no
long-lived tokens required. Configure it for each of the nine packages on
npmjs.com under **Settings → Publishing access**.

> **Note:** Trusted Publishing is only available after a package name already
> exists on npm. For brand-new package names, complete the
> [First Publish Bootstrap](#first-publish-bootstrap) first, then return here to
> configure Trusted Publishing.

Use these exact values for every package:

| Field | Value |
| ----- | ----- |
| Organization or user | `sandyfzu` |
| Repository | `xshot` |
| Workflow filename | `release.yml` |
| Environment name | `npm-production` |

Each npm package can have only one trusted publisher at a time. npm does not
validate the configuration when it is saved — a misconfigured field is only
discovered when `npm publish` attempts the OIDC exchange.

### Additional Requirements

- Keep `package.json` `repository.url` set to
  `git+https://github.com/sandyfzu/xshot.git`. npm Trusted Publishing verifies
  this during publish.
- Use GitHub-hosted runners for the publish job. npm Trusted Publishing does not
  support self-hosted runners.
- Do not add `NPM_TOKEN` or `NODE_AUTH_TOKEN` to the `npm-production`
  environment. OIDC authentication is exchanged automatically during
  `npm publish`.
- Keep `package.json`, `Cargo.toml`, `Cargo.lock`, and generated NAPI outputs
  committed before tagging.
- After the first successful Trusted Publishing release, set each package's
  Publishing access to **Require two-factor authentication and disallow tokens**
  and revoke any unused automation tokens.

---

## First Publish Bootstrap

npm Trusted Publishing requires a package to already exist on npm before its
settings page is available. For brand-new package names, use this one-time token
bootstrap path to create the names, then switch permanently to Trusted
Publishing.

The release workflow has an explicit `publish_auth` input to make this
distinction clear and prevent accidental fallback to tokens in normal releases:

- **`trusted-publishing`** — the normal tokenless OIDC path, attached to the
  `npm-production` environment.
- **`token-bootstrap`** — a one-time first-publish path, attached to the
  separate `npm-bootstrap` environment and not granted `id-token: write`.

Use `token-bootstrap` only when none of the nine package names exist on npm yet.

### Bootstrap Setup

1. **Create a GitHub Environment named `npm-bootstrap`:**
   - Go to **Settings → Environments → New environment**.
   - Name it `npm-bootstrap`.
   - Click **Configure environment** to open the environment settings page.
   - Under **Deployment protection rules**, select **Required reviewers**, add
     at least one reviewer, optionally enable **Prevent self-review**, then
     click **Save protection rules**.
     (See the note in the `npm-production` section above if this option is
     not visible.)
   - Add a **Deployment branches and tags** rule: select **Tag**, enter `v*`,
     then click **Add rule**.

2. **Create a short-lived npm granular access token on npmjs.com:**
   - **Expiration:** one day (minimum practical window).
   - **Access:** Read/Write. Because the package names do not yet exist, npm may
     require **All packages** read/write access for the bootstrap token.
   - **2FA bypass:** enable only if npm would otherwise block non-interactive
     publishing.
   - Keep the token active only for the duration of this bootstrap run.

3. **Store the token as an environment secret:**
   - Go to **Settings → Environments → npm-bootstrap → Environment secrets**.
   - Click **Add secret**, name it `NPM_TOKEN`, and paste the token value.
   - Do **not** add this secret to the `npm-production` environment or as a
     repository secret.

### Bootstrap Publish

1. Push the release tag to trigger the workflow in dry-run mode (see
   [Tag Workflow Dry Run](#tag-workflow-dry-run)) and wait for all jobs to pass.
2. Review the `npm-release-tarballs` artifact and the package job logs.
3. Manually dispatch the `Release` workflow from the same tag using the steps in
   [Manually Running the Release Workflow](#manually-running-the-release-workflow),
   with these inputs:
   - `publish`: `true`
   - `npm_tag`: `latest` (or a prerelease tag if this is a prerelease)
   - `publish_auth`: `token-bootstrap`
4. When the workflow pauses for environment approval, verify the exact tag,
   version, tarball count, and smoke-test results, then click
   **Approve and deploy**.

### Immediate Bootstrap Cleanup

After the first publish succeeds, complete these steps immediately:

1. **Delete the npm token** on npmjs.com under **Access Tokens → Revoke**.
   Revocation can take up to an hour to propagate — start it right away.
2. **Delete the `NPM_TOKEN` secret** from GitHub:
   **Settings → Environments → npm-bootstrap → Environment secrets → Delete**.
3. **Configure npm Trusted Publishing** for all nine packages as described in
   [npm Trusted Publishing](#npm-trusted-publishing).
4. **All future releases must use `publish_auth: trusted-publishing`.**
5. After one successful Trusted Publishing release, set each package's Publishing
   access to **Require two-factor authentication and disallow tokens**.

If a future Trusted Publishing run fails with an authentication error, fix the
npm Trusted Publisher configuration — do not fall back to the bootstrap token
path unless it is a deliberate emergency procedure with a new short-lived token
and the same environment approval controls.

---

## Dry Runs

Four dry-run layers answer different questions. No single layer proves
everything. Run all of them before a stable publish.

### Local Package Dry Run

Run before tagging to verify the package layout without writing to npm:

```bash
npm run create:npm-dirs -- --dry-run
npm run prepublish:napi -- --dry-run
npm pack --dry-run --json
```

**What this proves:**

- `napi create-npm-dirs` can derive platform package directories.
- `napi prepublish` can derive package metadata changes.
- Root package contents are correct: `index.js`, `index.d.ts`, `README.md`,
  `CHANGELOG.md`, `RELEASE.md`, `LICENSE`, `package.json`. No `.node` binary.

**What this does not prove:**

- Native targets build successfully.
- Platform tarballs contain binaries.
- npm authentication works.

### Local Root Tarball Publish Dry Run

Pack into a temporary directory and dry-run publishing the root tarball:

```bash
release_dir="$(mktemp -d)"
npm pack --pack-destination "$release_dir" --json
npm publish "$release_dir/xshot-$(node -p "require('./package.json').version").tgz" --dry-run --tag next --registry https://registry.npmjs.org
rm -rf "$release_dir"
```

Use `next` as the tag so this command is clearly a rehearsal. npm may print an
unauthenticated dry-run warning — this is expected and does not indicate a
problem. This validates tarball format and publish command shape only; platform
tarballs require CI-built artifacts.

### Tag Workflow Dry Run

Push the release tag to trigger the workflow automatically in non-publishing
mode, or manually dispatch with `publish: false`:

```bash
git tag v0.9.0
git push origin v0.9.0
```

**What this proves:**

- All eight native targets build.
- Platform packages are generated correctly.
- Root `optionalDependencies` point to all eight platform packages at the
  release version.
- Every platform package contains exactly one `.node` file.
- All nine tarballs pass `npm publish --dry-run`.
- Smoke tests pass on native and musl targets.

**What this does not prove:**

- A real npm registry publish. No package version is created by a dry run.
- OIDC authentication is not tested — only a real `npm publish` exercises it.

### CI Publish Dry Run

The `package` job runs `npm publish --dry-run` against all nine tarballs (in
publish order: platform packages first, root last) before any artifacts are
uploaded. If this step fails, fix the package layout or metadata before
proceeding. Do not bypass it.

---

## Release Flow

### 1. Prepare the Version

```bash
npm version <patch|minor|major|prerelease>
```

The `version` lifecycle script runs `napi version` to keep all Rust crate
versions aligned with `package.json`. Review the resulting diff before tagging.

### 2. Run Local Checks

All of these must pass before creating a tag:

```bash
npm ci
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
npx napi build --platform --release -p xshot -o . -- --locked
npm run typecheck
npm test
npm audit --audit-level=moderate
cargo deny check
npm pack --dry-run --json
npm run create:npm-dirs -- --dry-run
npm run prepublish:napi -- --dry-run
```

### 3. Tag the Commit

The tag must exactly match the `package.json` version. For version `0.9.0`:

```bash
git tag v0.9.0
git push origin v0.9.0
```

Pushing the tag automatically triggers the release workflow in non-publishing
mode. This is the full release-candidate dry run: all targets build, tarballs
are generated, `npm publish --dry-run` runs against every tarball, and smoke
tests run. No package version is created on npm, and OIDC authentication is only
exercised by a real publish.

### 4. Review Release Artifacts

After the tag workflow completes successfully:

1. Go to the workflow run on GitHub → **Artifacts** → download
   `npm-release-tarballs`.
2. Confirm the archive contains exactly **nine tarballs** (one root + eight
   platform packages).
3. Confirm the root tarball includes: `index.js`, `index.d.ts`, `README.md`,
   `CHANGELOG.md`, `RELEASE.md`, `LICENSE`, `package.json`. No `.node` file.
4. Confirm each platform tarball contains exactly one `.node` file.
5. Confirm the `package` job's `npm publish --dry-run` step shows all nine
   tarballs accepted.

### 5. Publish From the Tag

Dispatch the `Release` workflow from the same tag with `publish: true`. See
[Manually Running the Release Workflow](#manually-running-the-release-workflow)
for step-by-step instructions.

Input values for a **stable release**:

- `publish`: checked (`true`)
- `npm_tag`: `latest`
- `publish_auth`: `trusted-publishing`

Input values for a **prerelease**:

- `publish`: checked (`true`)
- `npm_tag`: `next`, `beta`, or `alpha` (match the prerelease identifier in the
  version string)
- `publish_auth`: `trusted-publishing`

### 6. Approve the Environment

After dispatch, the workflow pauses at the `npm-production` environment gate.
Review the workflow run to confirm it started from the correct tag, then click
**Approve and deploy** in the pending deployment banner.

### 7. Verify npm Publishing

After the publish job completes, verify all nine packages appear in the registry:

```bash
npm info xshot versions --json
npm info xshot-darwin-arm64 versions --json
npm info xshot-darwin-x64 versions --json
npm info xshot-linux-x64-gnu versions --json
npm info xshot-linux-arm64-gnu versions --json
npm info xshot-linux-x64-musl versions --json
npm info xshot-linux-arm64-musl versions --json
npm info xshot-win32-x64-msvc versions --json
npm info xshot-win32-arm64-msvc versions --json
```

Each command should list the new version.

### 8. Verify the GitHub Release

Navigate to **Releases** on the repository. The workflow creates a GitHub release
for the tag and uploads all nine tarballs. Confirm they are present.

### 9. End-to-End Verification

From a clean directory with no local tarball files, install from npm and verify
the module loads:

```bash
mkdir /tmp/xshot-verify && cd /tmp/xshot-verify
npm init -y
npm install xshot
node -e "const x = require('xshot'); console.log(Object.keys(x))"
```

The `Object.keys(x)` output should list all six public async functions.

---

## Manually Running the Release Workflow

The `Release` workflow uses `workflow_dispatch` with three inputs. You must
dispatch from the release tag — not from `main` or any branch. The workflow
refuses a publish dispatch from a branch.

### Using the GitHub Web UI

1. Go to the repository on GitHub:
   `https://github.com/sandyfzu/xshot`
2. Click the **Actions** tab at the top of the page.
3. In the left sidebar, under **Workflows**, click **Release**.
4. On the right side of the page, click the **Run workflow** button. A form
   drops down.
5. Change the **Branch** selector to the release tag:
   - Click the "Branch" dropdown (it defaults to the default branch).
   - Type the tag name in the search box, for example `v0.9.0`.
   - Tags are listed below branches — select the matching tag.
6. Fill in the three input fields:
   - **publish** — Check the box to publish to npm. Leave unchecked for a
     dry run (same as pushing a tag).
   - **npm_tag** — Select `latest` for stable releases, or `next`, `beta`,
     or `alpha` for prereleases.
   - **publish_auth** — Select `trusted-publishing` for normal releases.
     Select `token-bootstrap` only for the first-ever publish of new package
     names (see [First Publish Bootstrap](#first-publish-bootstrap)).
7. Click the green **Run workflow** button.

The new run appears in the Actions list within a few seconds. Click it to follow
progress. When it reaches the environment approval step, a yellow banner shows
**Review pending deployments** — click it to approve.

### Using the GitHub CLI

Install and authenticate the [GitHub CLI](https://cli.github.com/) first, then:

```bash
gh workflow run release.yml \
  --ref v0.9.0 \
  --field publish=true \
  --field npm_tag=latest \
  --field publish_auth=trusted-publishing
```

Replace `v0.9.0` with the actual release tag. To watch progress in the terminal:

```bash
gh run watch
```

To approve the pending environment deployment:

```bash
gh run list --workflow release.yml --limit 1
# note the run ID, then:
gh run review <run-id> --approve
```

---

## Release Workflow Jobs

### `prepare`

Validates that the tag matches `package.json` version, the semver shape is
valid, and a pre-release version is not being tagged as `latest`. Blocks publish
dispatches from branches.

### `verify`

Runs source-level gates once on Ubuntu:

- Rust format, Clippy, tests, and docs.
- TypeScript typecheck.
- npm audit.
- cargo-deny.

This job deliberately does not run the Node.js integration suite because those
tests load the native `.node` binding. Runtime loader coverage happens after the
native artifacts are built, packed, installed from tarballs, and smoke-tested.

### `build`

Builds all eight native targets:

- macOS x64 and arm64.
- Windows x64 and arm64.
- Linux GNU x64 and arm64 on Ubuntu 24.04.
- Linux musl x64 and arm64 (Alpine containers).

Linux arm64 builds run on GitHub-hosted arm64 runners. Linux musl builds run
inside Alpine containers to ensure musl toolchain coverage.

### `package`

Generates and validates npm packages from the build artifacts:

1. Downloads all native build artifacts.
2. Runs `npm run create:npm-dirs`, `npm run artifacts`, `npm run prepublish:napi`
   to finalize NAPI package metadata without publishing platform packages.
3. Validates `optionalDependencies`, package names, versions, and `.node` file
   placement.
4. Verifies the root tarball dry-run contents.
5. Packs all nine tarballs.
6. Runs `npm publish --dry-run` for all tarballs (platform packages first, root
   last) against the npmjs registry.
7. Uploads the tarballs as the `npm-release-tarballs` artifact.

### `smoke-native`

Installs the root tarball and the matching platform tarball on native runners for
macOS, Windows, and Linux GNU. Linux GNU tarballs are smoked on Ubuntu 24.04,
the current prebuilt GNU/Linux baseline. Verifies:

- Package installation succeeds from tarballs.
- `require('xshot')` loads the native binding.
- Every function declared by the installed `index.d.ts` exists at runtime.
- Invalid format rejection returns `[INVALID_ARGUMENT]` through the documented
  message-prefix contract.

### `smoke-musl`

Runs the same smoke test in Alpine containers for Linux musl x64 and arm64.

### `publish_trusted`

Activated when `publish: true` and `publish_auth: trusted-publishing`.

- Uses npm Trusted Publishing via GitHub Actions OIDC.
- Runs on `environment: npm-production`.
- Grants `id-token: write` for the OIDC token exchange.
- Verifies Node.js and npm meet the Trusted Publishing minimum versions.
- Refuses to publish if the root package version already exists on npm.
- Publishes platform packages first, then the root package.
- npm generates provenance automatically for public packages in public
  repositories; the workflow does not pass `--provenance` explicitly.

Do not add `NPM_TOKEN`, `NODE_AUTH_TOKEN`, or `npm whoami` to this job.

### `publish_token_bootstrap`

Activated when `publish: true` and `publish_auth: token-bootstrap`.

- Uses `environment: npm-bootstrap` and the `NPM_TOKEN` environment secret.
- Does not request `id-token: write`.
- Writes a temporary npm user config under `RUNNER_TEMP` with strict permissions
  (`umask 077`).
- Removes the temporary config before the job exits.
- Publishes the same smoke-tested tarballs as the Trusted Publishing path.

This job exists only to create package names for the first publish. Delete the
token immediately after the bootstrap run.

### `publish_complete`

Gate job that confirms the selected publish path succeeded before the GitHub
release job runs. If the selected auth path was skipped, failed, or cancelled,
the workflow stops here and no GitHub release is created or updated.

### `github-release`

Creates or updates the GitHub release for the tag and uploads all nine release
tarballs.

---

## Failure and Recovery

### Tag or Version Mismatch

The `prepare` job fails if the tag does not match `package.json`. Fix by
creating a tag that matches the committed version. Do not move a tag to a
different commit after a failed publish attempt.

### Missing Native Artifact

The `package` job fails before any publish step if a build artifact is missing.
Fix the build failure, rerun the workflow, and verify the artifact again.

### Broken Package Metadata

If `optionalDependencies`, package names, versions, or `.node` file placement
are wrong, the `package` job fails before publishing. Fix the NAPI configuration
or workflow. Do not edit generated loader files by hand — regenerate them using
the NAPI-RS CLI.

### Partial npm Publish

The workflow publishes the root package last. If some platform packages were
published but the root package was not, fix the external issue and rerun the
workflow from the same tag. The workflow skips platform packages that already
exist at that version. If the root package already exists at that version, npm
will reject republishing — prepare a new patch release instead.

### Trusted Publishing Authentication Failure

If npm returns `ENEEDAUTH` or an OIDC error, verify each field before rerunning:

| Check | Expected value |
| ----- | -------------- |
| Trusted publisher configured for every package | Yes |
| Organization or user | `sandyfzu` |
| Repository | `xshot` |
| Workflow filename | `release.yml` (not the full path) |
| Environment name | `npm-production` |
| Runner type | GitHub-hosted (not self-hosted) |
| `id-token: write` granted to publish job | Yes |
| `repository.url` in `package.json` | `git+https://github.com/sandyfzu/xshot.git` |

Do not debug with `npm whoami` — OIDC authentication is scoped to the publish
operation and `whoami` does not reflect OIDC status.

### Token Bootstrap Authentication Failure

If the bootstrap job fails before publishing:

1. Confirm the run used `publish_auth: token-bootstrap`.
2. Confirm the `npm-bootstrap` environment was approved.
3. Confirm `NPM_TOKEN` exists as an environment secret on `npm-bootstrap`.
4. Confirm the token has sufficient access (new packages may require **All
   packages** read/write access).
5. Confirm the token is not expired.
6. Confirm npm 2FA is not blocking non-interactive publishing, or the token is
   configured to bypass 2FA.

If some platform packages published and a later one failed, rerun from the same
tag after fixing the issue. The workflow skips packages that already exist and
refuses to republish the root package if it exists.

### npm Package Name Rejection

If npm rejects unscoped package names such as `xshot-darwin-arm64` (spam
detection), move platform packages under an owned npm scope by updating the
NAPI-RS package name configuration and regenerating NAPI outputs. Do not edit
generated loader files by hand.

---

## Stable Release Checklist

Complete every item before marking a release done.

- [ ] `package.json` and Rust manifests have the correct version.
- [ ] `CHANGELOG.md` has release notes for this version.
- [ ] `cargo fmt`, Clippy, Rust tests, Rust docs, TypeScript typecheck, Node
      tests, npm audit, and cargo-deny all pass.
- [ ] npm Trusted Publishing configured for all nine packages, or first-publish
      bootstrap path selected.
- [ ] Tag is exactly `v<package.json version>`.
- [ ] Tag-triggered workflow completed successfully without publishing.
- [ ] `npm-release-tarballs` artifact contains exactly nine tarballs.
- [ ] Root tarball has no `.node` binary.
- [ ] Platform tarballs each contain exactly one `.node` binary.
- [ ] `npm publish --dry-run` passed for all nine tarballs in CI.
- [ ] Smoke tests passed for native and musl installs.
- [ ] Manual publish dispatched from the tag with correct dist-tag and
      `publish_auth`.
- [ ] Environment approval granted after reviewing artifacts.
- [ ] All nine npm packages visible in the registry at the new version.
- [ ] GitHub release contains all nine tarballs.
- [ ] End-to-end install from npm succeeds in a clean directory.
- [ ] (Token-bootstrap only) `NPM_TOKEN` deleted from GitHub and revoked on
      npmjs.com immediately after the first publish.

---

## Reference

- [NAPI-RS release guidance](https://napi.rs/docs/deep-dive/release)
- [NAPI-RS artifact packaging](https://napi.rs/docs/cli/artifacts)
- [NAPI-RS prepublish command](https://napi.rs/docs/cli/pre-publish)
- [NAPI-RS create-npm-dirs command](https://napi.rs/docs/cli/create-npm-dirs)
- [NAPI-RS build command](https://napi.rs/docs/cli/build)
- [NAPI-RS configuration schema](https://napi.rs/docs/cli/napi-config)
- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers)
- [npm access tokens](https://docs.npmjs.com/about-access-tokens)
- [npm creating access tokens](https://docs.npmjs.com/creating-and-viewing-access-tokens)
- [npm revoking access tokens](https://docs.npmjs.com/revoking-access-tokens)
- [npm package manifest fields](https://docs.npmjs.com/cli/v11/configuring-npm/package-json)
- [npm pack](https://docs.npmjs.com/cli/v11/commands/npm-pack)
- [npm publish](https://docs.npmjs.com/cli/v11/commands/npm-publish)
- [GitHub Actions OIDC](https://docs.github.com/en/actions/security-for-github-actions/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
- [GitHub Actions environments](https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment)
- [GitHub Actions workflow syntax](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions)
- [GitHub Actions artifacts](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/storing-and-sharing-data-from-a-workflow)
- [GitHub Actions token permissions](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication)
- [GitHub-hosted runner labels](https://docs.github.com/en/actions/reference/github-hosted-runners-reference)
- [GitHub CLI workflow run](https://cli.github.com/manual/gh_workflow_run)
