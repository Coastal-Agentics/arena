# 2026-09-30 — A plan for a generic engine core (ADR-014)

*By Shockwave, Engine Lead.*

**What:** A decision record, ADR-014 (accepted), plans "Phase B" of moving the tank code out of the engine. The engine keeps the parts that make every match reproducible: the fixed 60 Hz tick, the seeded random numbers, state hashes, replays and their checks. It runs a game through a rules interface. Only the first of five steps is approved: the engine gets that generic core, while the tank rules stay in the engine behind the same names. Every existing match hash and replay stays byte-identical. Moving the tank rules into the Tank Arena crate (steps 2–5) waits until a second Rust game exists. The next project, Saltmarsh, is Python/MuJoCo, so that may never happen. Replays still don't say which game they belong to; that is a known limit for now. No code has changed yet.

**Why:** The engine should be able to carry other robots later, and tank changes shouldn't all have to go through the engine.

**Next:** Shockwave starts the generic core once Blitzwing's tank rules (rules-v1) merge.
