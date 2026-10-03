#!/usr/bin/env bash
# Rebuild the browser engine into web/pkg (committed; the site deploys with no build step).
# Needs: rustup target wasm32-unknown-unknown, and wasm-bindgen-cli at the SAME version
# as the wasm-bindgen crate pinned in engine-wasm/Cargo.toml:
#   cargo install wasm-bindgen-cli --version 0.2.100 --locked
# The toolchain is pinned in rust-toolchain.toml; a different rustc gives different bytes.
# CI (the `wasm` job) rebuilds and fails if web/pkg differs from what is committed.
set -euo pipefail
cd "$(dirname "$0")/.."
# Panic messages embed source paths, including the cargo registry under CARGO_HOME
# (/home/<user>/.cargo/...). Remap them so the output doesn't depend on who built it.
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${cargo_home%/}=/cargo --remap-path-prefix=$PWD=/src"
# One codegen unit for the wasm build only (native builds keep the default 16): LLVM
# then sees each crate whole and drops ~15 KB of duplicated and dead code. Same
# behaviour (every parity hash is identical) and no slower in Node.
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
cargo build -p engine-wasm --release --target wasm32-unknown-unknown
# --remove-name-section drops the debug `name` section (function names, ~70 KB) that only
# profilers and stack traces use; --remove-producers-section drops the toolchain telemetry.
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir web/pkg target/wasm32-unknown-unknown/release/engine_wasm.wasm
ls -l web/pkg
