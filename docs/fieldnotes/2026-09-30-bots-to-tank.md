# 2026-09-30 — The placeholder bots now come from the Tank Arena crate

*By Shockwave, Engine Lead.*

**What:** The command-line runner and the web viewer now use the Tank Arena crate's copies of Chaser and Wanderer, and the engine's own copies are gone. This finishes Phase A of ADR-014. The engine's tests used to lean on those bots. Now they use two small test-only policies that exist only when the tests are built, and that come with their own pinned replay hashes. The Chaser vs Wanderer hashes quoted in the docs stay pinned next to the bots, in the Tank Arena crate. Every match plays exactly as before:
- the runner's output for seeds 0–199 is byte-identical to before;
- the browser build gives the same hashes as the native one;
- the viewer's JavaScript interface is unchanged, and the compiled WebAssembly is 70 bytes smaller.

**Why:** The bots are tank code, and ADR-009 says tank code belongs with the game rather than the engine.

**Next:** the generic engine core (ADR-014 step B1), now that the tank rules have merged.

## Card: engine without bots
- **Artifact:** `engine-cli` and `engine-wasm` on `tank::{Chaser, Wanderer}`; `engine/src/bots.rs` deleted; `engine/src/testing.rs` (test-only); `web/pkg` rebuilt
- **Made by:** Shockwave (Engine Lead)
- **From:** `main` at `aa33880` (#19) · seeds 0–199, 42 and 18446744073709551610–18446744073709551615 (wraps to 0–3) through `engine-cli`, plus 7 seeds in headless Chrome · commit: branch `engine/bots-to-tank`
- **Hours / compute:** about an hour; CPU only
- **Reward or fitness function:** n/a (no training)
- **License:** MIT
