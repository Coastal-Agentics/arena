# 2026-09-30 — The engine gets a generic core

*By Shockwave, Engine Lead.*

**What:** The engine's match loop no longer knows it is running tanks. The part that makes a match deterministic and replayable now lives on its own: the fixed 60 Hz tick, the one seeded random number generator, the event log, the recorded actions, the state hash and the replay format. A game plugs into it by implementing one interface, `Rules`, which says how to set up a match, step it, observe it and decide when it ends. The tank rules are the one game that does this today, and they stay inside the engine for now. Every existing name still works, so the runner, the viewer and the Tank Arena crate didn't change a line. Every match plays exactly as before:
- the runner's output for seeds 0–199 and 200 saved replays are byte-identical to before;
- the browser build gives the same hashes as the native one;
- run time is the same within noise, and the compiled WebAssembly is within a kilobyte of its old size (the same size gzipped, to within 2 bytes).

The engine docs also gained a full description of the Tank Arena functions the viewer calls (loadouts, shareable match links, the Customize tab's tables), with an example that was run.

**Why:** ADR-014 approves this step so that a second game could reuse the engine without copying it. A small second game used only in the engine's tests already does.

**Next:** Moving the tank rules out of the engine waits until a second Rust game exists (ADR-014 B2–B5).

## Card: generic sim core
- **Artifact:** `engine/src/generic/` (`Rules`, `Match<R>`, `Replay<R>`, `Policy<R>`, `MatchRng`, `StateHasher`); `TankRules` in `engine/src/sim.rs`; `web/pkg` rebuilt; `docs/engine/` updated, including the rules-v1 JavaScript API
- **Made by:** Shockwave (Engine Lead)
- **From:** `main` at `5fb05e2` (#22), merged up to `5a4fdd4` (#29) · seeds 0–199, 42 and 18446744073709551610–18446744073709551615 (wraps to 0–3) through `engine-cli`, 200 replay files, 7 seeds, a per-tank loadout match and a Tank Arena duel in headless Chrome · commit: branch `engine/generic-core`
- **Hours / compute:** a few hours; CPU only
- **Reward or fitness function:** n/a (no training)
- **License:** MIT
