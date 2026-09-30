#!/usr/bin/env bash
# Runs tools/ci/check.sh in a Linux container, since CI runs on Linux and
# allocation counts, paths and toolchain behaviour can differ from macOS.
# Usage: tools/ci/check-linux.sh [linux/amd64|linux/arm64]
set -euo pipefail
cd "$(dirname "$0")/../.."
platform=${1:-linux/amd64}
docker volume create milsymbol-target >/dev/null
docker run --rm --platform "$platform" \
  -v "$PWD":/src -v milsymbol-target:/target -v milsymbol-cargo:/usr/local/cargo/registry \
  -w /src -e CARGO_TARGET_DIR=/target rust:1 \
  sh -c 'apt-get update -qq >/dev/null && apt-get install -y -qq jq >/dev/null && rustup component add rustfmt clippy >/dev/null 2>&1; tools/ci/check.sh'
