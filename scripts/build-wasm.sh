#!/usr/bin/env bash
# Rebuild the browser engine into web/pkg (committed; the site deploys with no build step).
# Needs: rustup target wasm32-unknown-unknown, and wasm-bindgen-cli at the SAME version
# as the wasm-bindgen crate pinned in engine-wasm/Cargo.toml:
#   cargo install wasm-bindgen-cli --version 0.2.100 --locked
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p engine-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
  target/wasm32-unknown-unknown/release/engine_wasm.wasm
ls -l web/pkg
