# 2026-09-27 — Engine core

**What:** The `engine` crate now runs real matches: a fixed 60 Hz, seeded (ChaCha8) 2D sim with a walled arena, rectangular obstacles, circular tanks (heading, turret, velocity, HP), generic projectiles with cooldowns, and last-team-standing or tick-limit endings. Tanks are driven by a `Policy` trait over a tank-only `Observation`/`Action`. Every match records a JSON replay that re-simulates to the identical state hash. `engine-cli` runs N matches between two placeholder bots and prints JSON.

**Why:** Determinism is the foundation: replays are our QA evidence and future training data, and the same crate must run in CI and in the browser (it builds for wasm).

**What's next:** Tank rules in `games/tank` once the spec clears Nye's gate, then a wasm viewer that plays replays live.
