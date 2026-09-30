# 2026-09-30 — Engine doc fixes and config-checked replays

*By Shockwave, Engine Lead.*

**What:** Replays now record a setup hash of the seed and config, so editing a replay's config fails verification even when the match would play out the same. Before, deleting every obstacle from the seed-7 duel still verified. Replay format is now 3; format 2 files still load and verify without the config check. Match state hashes are unchanged. ADR-002 and ADR-009 carry dated corrections: the viewer draws from plain JavaScript, and the tank code lives in `engine/` today while `games/tank` is a stub. `engine-cli --help` now says Coastal Agentics Arena. STATE notes that the placeholder bots are lopsided.

**Why:** The engine docs found these gaps; the docs and the verifier should both be true.

**Next:** Tank Arena rules once GATE-002 clears; Blitzwing's policies can fix the bot balance.
