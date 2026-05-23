#!/usr/bin/env bash
set -euo pipefail

: "${VERSION:?VERSION is required}"
: "${NPM_TAG:?NPM_TAG is required}"

command -v npm >/dev/null 2>&1 || {
  echo "npm is required to publish release tarballs." >&2
  exit 1
}

command -v node >/dev/null 2>&1 || {
  echo "Node.js is required to verify tarball integrity before publishing." >&2
  exit 1
}

validate_version() {
  local semver_pattern='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-((0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(\.(0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?(\+([0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*))?$'

  if [[ ! "$VERSION" =~ $semver_pattern ]]; then
    echo "VERSION must be valid SemVer 2.0.0, got: $VERSION" >&2
    exit 1
  fi
}

validate_npm_tag() {
  case "$NPM_TAG" in
    latest | next | beta | alpha) ;;
    *)
      echo "NPM_TAG must be one of: latest, next, beta, alpha. Got: $NPM_TAG" >&2
      exit 1
      ;;
  esac
}

validate_version
validate_npm_tag

npm_registry="https://registry.npmjs.org"
dist_dir="./dist"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

if [[ ! -d "$dist_dir" ]]; then
  echo "Missing dist directory with release tarballs." >&2
  exit 1
fi

root_package="captura"
root_tarball="${dist_dir}/${root_package}-${VERSION}.tgz"
platform_packages=(
  captura-darwin-x64
  captura-darwin-arm64
  captura-win32-x64-msvc
  captura-win32-arm64-msvc
  captura-linux-x64-gnu
  captura-linux-arm64-gnu
  captura-linux-x64-musl
  captura-linux-arm64-musl
)

expected_tarballs=("$root_tarball")
for package_name in "${platform_packages[@]}"; do
  expected_tarballs+=("${dist_dir}/${package_name}-${VERSION}.tgz")
done

validate_tarballs() {
  local tarball
  for tarball in "${expected_tarballs[@]}"; do
    if [[ ! -f "$tarball" ]]; then
      echo "Missing expected release tarball: $tarball" >&2
      exit 1
    fi
  done

  local actual_tarballs_file
  local expected_tarballs_file
  actual_tarballs_file="$(mktemp "${tmp_dir}/actual-tarballs.XXXXXX")"
  expected_tarballs_file="$(mktemp "${tmp_dir}/expected-tarballs.XXXXXX")"

  find "$dist_dir" -maxdepth 1 -type f -name '*.tgz' -print | sort >"$actual_tarballs_file"
  printf '%s\n' "${expected_tarballs[@]}" | sort >"$expected_tarballs_file"

  if ! cmp -s "$expected_tarballs_file" "$actual_tarballs_file"; then
    echo "Release tarballs in $dist_dir do not match the exact expected set." >&2
    echo "Expected:" >&2
    sed 's/^/  /' "$expected_tarballs_file" >&2
    echo "Actual:" >&2
    sed 's/^/  /' "$actual_tarballs_file" >&2
    exit 1
  fi
}

tarball_integrity() {
  local tarball="$1"

  node - "$tarball" <<'NODE'
  const { createHash } = require('node:crypto')
  const { readFileSync } = require('node:fs')

  const tarball = process.argv[2]
  const digest = createHash('sha512').update(readFileSync(tarball)).digest('base64')
  process.stdout.write(`sha512-${digest}`)
NODE
}

published_integrity() {
  local spec="$1"
  local stderr_file
  local stdout_file
  stderr_file="$(mktemp "${tmp_dir}/npm-view-stderr.XXXXXX")"
  stdout_file="$(mktemp "${tmp_dir}/npm-view-stdout.XXXXXX")"

  if npm view "$spec" dist.integrity --registry "$npm_registry" >"$stdout_file" 2>"$stderr_file"; then
    local integrity
    integrity="$(tr -d '\r\n' <"$stdout_file")"
    if [[ -z "$integrity" || "$integrity" == "undefined" ]]; then
      echo "npm registry returned no dist.integrity for $spec. Aborting before publishing." >&2
      return 2
    fi

    printf '%s\n' "$integrity"
    return 0
  fi

  if grep -Eqi '(E404|404 Not Found|not found)' "$stderr_file"; then
    return 1
  fi

  cat "$stderr_file" >&2
  echo "Unable to determine whether $spec already exists on npm. Aborting before publishing." >&2
  return 2
}

validate_tarballs

if published_integrity "${root_package}@${VERSION}" >/dev/null; then
  echo "${root_package}@${VERSION} already exists on npm. Refusing to republish root package." >&2
  exit 1
else
  status="$?"
  if [[ "$status" != "1" ]]; then
    exit "$status"
  fi
fi

for package_name in "${platform_packages[@]}"; do
  tarball="${dist_dir}/${package_name}-${VERSION}.tgz"
  package_spec="${package_name}@${VERSION}"

  published_platform_integrity=""
  if published_platform_integrity="$(published_integrity "$package_spec")"; then
    local_platform_integrity="$(tarball_integrity "$tarball")"
    if [[ "$published_platform_integrity" != "$local_platform_integrity" ]]; then
      echo "$package_spec already exists on npm, but its integrity does not match $tarball." >&2
      echo "Published integrity: $published_platform_integrity" >&2
      echo "Local integrity:     $local_platform_integrity" >&2
      echo "Refusing to publish the root package against a mismatched platform package." >&2
      exit 1
    fi

    echo "$package_spec already exists on npm with matching integrity; leaving existing platform package in place."
    continue
  else
    status="$?"
    if [[ "$status" != "1" ]]; then
      exit "$status"
    fi
  fi

  npm publish "$tarball" --tag "$NPM_TAG" --registry "$npm_registry"
done

npm publish "$root_tarball" --tag "$NPM_TAG" --registry "$npm_registry"
