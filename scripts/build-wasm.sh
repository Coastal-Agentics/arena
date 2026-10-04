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

fail() {
  echo "build-wasm.sh: error: $*" >&2
  exit 1
}

# Package the .wasm cargo actually built, wherever its target directory is
# (CARGO_TARGET_DIR, CARGO_BUILD_TARGET_DIR or build.target-dir), not a hard-coded
# target/. `cargo metadata` resolves it; sed reads the one field (no jq needed).
target_dir="$(cargo metadata --format-version 1 --no-deps |
  sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
[ -n "$target_dir" ] || fail "could not read target_directory from cargo metadata"
expected="$target_dir/wasm32-unknown-unknown/release/engine_wasm.wasm"

started="$(mktemp)" # mtime = build start
trap 'rm -f "$started"' EXIT
# Diagnostics and progress still go to the terminal; the JSON messages on stdout say
# which file cargo produced and whether it was rebuilt ("fresh": false) or up to date.
messages="$(cargo build -p engine-wasm --release --target wasm32-unknown-unknown \
  --message-format=json-render-diagnostics)"
artifact="$(printf '%s\n' "$messages" |
  grep '"reason":"compiler-artifact"' | grep '"name":"engine_wasm"' || true)"
[ -n "$artifact" ] || fail "cargo reported no engine_wasm artifact"
wasm="$(printf '%s\n' "$artifact" | grep -o '"[^"]*engine_wasm\.wasm"' | tr -d '"' || true)"
[ "$wasm" = "$expected" ] ||
  fail "cargo built '${wasm:-no .wasm}', expected '$expected' (target dir from cargo metadata)"
[ -f "$wasm" ] || fail "$wasm is missing after the build"
# Rebuilt: the file must be newer than the build start. Up to date ("fresh": true):
# cargo has just checked it against every source, so an older mtime is expected.
if printf '%s\n' "$artifact" | grep -q '"fresh":false'; then
  [ "$wasm" -nt "$started" ] || fail "$wasm is older than this build; refusing to package it"
fi
echo "build-wasm.sh: packaging $wasm"

# --remove-name-section drops the debug `name` section (function names, ~70 KB) that only
# profilers and stack traces use; --remove-producers-section drops the toolchain telemetry.
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir web/pkg "$wasm"
ls -l web/pkg
