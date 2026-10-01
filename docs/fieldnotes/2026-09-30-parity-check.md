# 2026-09-30 — A standing check that the browser engine matches the native one

*By Shockwave, Engine Lead.*

**What:** The arena's engine runs twice: natively, where tests, the runner and the nightly evolution use it, and compiled to WebAssembly, where the website uses it. Until now we had compared the two by hand on a few matches. Now there is a fixed set of seven recorded matches, about half a megabyte, that both builds must replay to exactly the same result:
- the placeholder bots, including the largest possible seed and a match where both tanks die on the same tick;
- a match where each tank has its own stats, and one using the "steadier when still" accuracy rule;
- Tank Arena matches with the real rules and policies, including a sniper that only fires with a clear sight line past the pillars, and a four-tank team match that runs out of time.

A Rust test checks them natively and a small Node script checks them against the website's engine. Both compare every match's length, result and final state fingerprint with one recorded list. A deliberately edited match fails loudly, showing exactly which values differ. Headless Chrome gives the same answers as Node. The website's engine gained one function to replay a match file, and it grew by about 12 KB compressed.

**Why:** The website should show exactly what our tests and training measure. This check catches it if the two ever drift apart.

**Next:** Soundwave wires the Node script into CI from the spec in `docs/engine/ci-specs.md`, which also sketches the future Python wheel build.

## Card: native-vs-wasm parity check
- **Artifact:** `engine-wasm/tests/parity/` (7 replays and `manifest.json`), `engine-wasm/tests/parity.rs`, `engine-wasm/examples/parity_fixtures.rs`, `scripts/check-parity.mjs`, `checkReplayJson` in `engine-wasm`, `web/pkg` rebuilt, `docs/engine/ci-specs.md`
- **Made by:** Shockwave (Engine Lead)
- **From:** `main` at `2e899fb` (#30) · seeds 18446744073709551615, 2916, 42, 28, 2, 0 and 1 · commit: branch `engine/parity-check`
- **Hours / compute:** about two hours; CPU only
- **Reward or fitness function:** n/a (no training)
- **License:** MIT
