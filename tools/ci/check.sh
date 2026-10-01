#!/usr/bin/env bash
# The CI `native` job, runnable locally: CI calls this script, so a passing
# local run means the job passes. Use tools/ci/check-linux.sh to run it on
# Linux, where CI runs.
set -euo pipefail
export RUSTFLAGS="${RUSTFLAGS:--D warnings}"
docflags="-D warnings -D missing_docs -D rustdoc::broken_intra_doc_links"
step() { echo "::group::$*"; "$@"; echo "::endgroup::"; }
step cargo fmt --all --check
step cargo clippy --workspace --all-targets -- -D warnings
step cargo clippy -p milsymbol --all-targets --all-features -- -D warnings
step cargo clippy -p milsymbol --all-targets --no-default-features --features compact-paths -- -D warnings
step cargo test --workspace --all-targets
step cargo test -p milsymbol --all-targets --all-features
step cargo test --no-default-features
step cargo test -p milsymbol --no-default-features --features compact-paths
step cargo test --release --test corpus
step cargo test --release -p milsymbol --features compact-paths --test corpus
step cargo test --release -p milsymbol --features compact-paths --test allocations --test compact_paths
step cargo test --doc
RUSTDOCFLAGS="$docflags" step cargo doc --no-deps -p milsymbol --all-features
RUSTDOCFLAGS="$docflags" step cargo doc --no-deps -p milsymbol --no-default-features
step cargo build --release
step tools/ci/file-size.sh
step tools/ci/lint-config.sh
step tools/ci/changelog.sh
