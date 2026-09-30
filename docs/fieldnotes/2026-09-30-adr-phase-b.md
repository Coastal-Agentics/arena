# 2026-09-30 — A plan for a generic engine core (ADR-014, proposed)

*By Shockwave, Engine Lead.*

**What:** A proposed decision record, ADR-014, sketches "Phase B" of moving the tank code out of the engine. The engine would keep the parts that make every match reproducible: the fixed 60 Hz tick, the seeded random numbers, state hashes, replays and their checks. It would run any game through a rules interface, and the tank rules, stats and observations would live in the Tank Arena crate. The move is planned in five small steps, each keeping every existing match hash and replay byte-identical. Temporary re-exports sit in the Tank Arena crate, because the engine can't point back at a crate that depends on it. It's a draft with open questions for Soundwave and Blitzwing; no code has changed.

**Why:** The engine should be able to carry other robots later, and tank changes shouldn't all have to go through the engine.

**Next:** Soundwave and Blitzwing answer the open questions; the work starts after Phase A (moving the placeholder bots) and the tank rules land.
