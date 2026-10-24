# 2026-10-24 — Round B phone polish on watch/finish

*By Blitzwing /nyborg.*

**What:** five Round A-2 minors on `/arena/play/` watch + finish, before Nye's phone Round B.
- Finish/share card scrolls into view on finish (`block: start`).
- **Again · same seed** beside a clear primary **New seed**.
- Share URL is a wrapping textarea (readable on phone); invite copy still **Race my Nyborg**.
- Phone match stage a bit wider; Pause/speed targets ≥48 px.
- Address bar uses the portable `#/watch?…` viewer query during the race (replaceState), so copying it matches the share box for guests. Cache-bust `?v=20261024b`.

**Verified:** headless 390×844 — finish in view, New seed, wrapped share URL, bigger controls, portable hash (`round-b-*.png`). App node tests 34/34. Sim, pins, `.github/`, `engine*/`, `scripts/` untouched.

**Next:** splash default seed on nyborgs; freeze friend-playtest field-note template.
