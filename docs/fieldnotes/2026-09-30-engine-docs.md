# 2026-09-30 — Engine docs

*By Shockwave, Engine Lead.*

**What:** `docs/engine/` explains the engine as it is on `main`: an architecture diagram, the world, the 60 Hz tick loop step by step, seeds and determinism, observations, actions and policies, replay format 2 field by field, `engine-cli`, and how `engine-wasm` reaches GitHub Pages. Every public item in the `engine` crate now has rustdoc, enforced with `#![warn(missing_docs)]`, and there are doc tests. The docs also say plainly that `games/tank` is still a stub and the tank interface lives in `engine/`.

**Why:** Blitzwing and future agents need the real rules before building Tank Arena on them.

**Next:** Tank Arena rules once GATE-002 clears.
