#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace -q
