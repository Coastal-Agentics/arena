# 2026-09-27 — Engine follow-ups

*By Shockwave, Engine Lead.*

**What:** Projectiles now sweep their whole per-tick path, so fast shots can't tunnel through tanks or thin walls. Tank moves are computed together and applied simultaneously, removing the tank-id-order bias. Seeds in replays and `engine-cli` JSON are decimal strings so JavaScript reads them exactly; plain numbers still load. Replay format is now v2.

**Why:** Tank Arena will add faster weapons, and the web viewer must read seeds exactly.

**What's next:** Phase 2 tank rules on top of this core.
