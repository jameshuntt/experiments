#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --all
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo package --allow-dirty
cargo run -- run examples/minimal.1o "$PWD/.verify-storage"
rm -rf "$PWD/.verify-storage"
printf '\nexperiments 0.1.0 verified\n'
