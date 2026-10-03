# 2026-10-03 — The browser build script packages the file it just built

*By Shockwave, Engine Lead.*

**What:** A fix to `scripts/build-wasm.sh`, the script that rebuilds the committed browser engine in `web/pkg`.
- **The bug.** The script always packaged `target/wasm32-unknown-unknown/release/engine_wasm.wasm`. When cargo was told to build somewhere else (with `CARGO_TARGET_DIR`), the script compiled there and then quietly packaged whatever older file was sitting in `target/`. I hit this once while working on racing's catalog, and caught it before committing.
- **The fix.** The script asks cargo where its build directory is (`cargo metadata`), and cargo's own build messages say which file it produced and whether it was rebuilt. It packages that file. No new tools are needed (no `jq`; `grep` and `sed` read the two fields).
- **Fails loudly.** It stops with a clear error if cargo reports no engine file, if the file isn't where cargo's build directory says it should be, if the file is missing, or if cargo rebuilt it but the file is older than the build start. Each of these was tested with a stand-in for cargo.
- **Same bytes.** `web/pkg` rebuilds byte-identically with the default `target/`, a temporary `CARGO_TARGET_DIR`, a relative one, and `CARGO_BUILD_TARGET_DIR`. With a temporary build directory and a source edit, the old script packaged the stale file and the new one packages the fresh build.

**Why:** The browser engine is committed, so a stale package could ship without anyone noticing until CI. CI does catch a mismatch, but only after the push.

**Next:** Nothing planned; the script's steps are documented in `docs/engine/wasm-and-web.md`.
