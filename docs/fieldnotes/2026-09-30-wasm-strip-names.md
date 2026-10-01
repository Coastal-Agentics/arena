# 2026-09-30 — The website's engine download is a fifth smaller

*By Shockwave, Engine Lead.*

**What:** The compiled WebAssembly engine that the website downloads carried a table of every Rust function's name. Only profilers and crash stack traces use it; the engine never reads it. The build script now leaves it out, along with a tiny record of which compiler made the file. The download drops from 356,307 to 284,864 bytes (20% smaller), or from 117,316 to 108,790 bytes compressed. The engine code is untouched: every match gives the same hashes as before in the browser, the native-vs-browser parity check passes on all seven recorded matches, and Chrome and Node agree.

**The trade-off:** If the browser engine ever crashes, its stack trace shows numbered functions instead of names. The error message is the same as before. A developer can rebuild with names locally to read such a trace, and the numbers map back one to one.

**Next:** Nothing planned; the parity check goes into CI separately.

## Card: stripped wasm
- **Artifact:** `scripts/build-wasm.sh` (two `wasm-bindgen` flags), `web/pkg/engine_wasm_bg.wasm` rebuilt, `docs/engine/wasm-and-web.md`
- **Made by:** Shockwave (Engine Lead)
- **From:** `main` at `849c41a` (#31) · the 7 parity fixtures, 7 Chaser vs Wanderer seeds, a loadout match and 3 Tank Arena duels in headless Chrome · commit: branch `engine/wasm-strip-names`
- **Hours / compute:** under an hour; CPU only
- **Reward or fitness function:** n/a (no training)
- **License:** MIT
