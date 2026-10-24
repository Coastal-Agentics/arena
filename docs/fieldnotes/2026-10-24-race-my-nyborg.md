# 2026-10-24 — Race my Nyborg share pack

*By Blitzwing /nyborg.*

**What:** Round A blockers on `/arena/play/` so a friend can open a race from a phone.
- Start CTA hardening: `el()` never stringifies a function into text/attrs; setup uses only `startLabel` (short fallback). Cache-bust `?v=20261024a2` on the play entry and app/games imports so Pages cannot mix stale modules after #112.
- Finish share pack: invite copy exactly **Race my Nyborg**, with a visible `#/watch?…` viewer-query link (cars/looks/nameN) — not `?slots=<id>`, which drops unknown ids to house-only for a guest.
- Guest cold-open keeps the sharer's lineup with an empty library.

**Verified:** headless 390×844 — Start is "Start the race ▶"; finish shows Race my Nyborg + viewer-query URL; guest plays Pip + house (`round-a2-*.png`). App node tests 13/13. Sim, pins, `.github/`, `engine*/`, `scripts/` untouched.

**Next:** splash default seed on nyborgs; freeze friend-playtest field-note template. Still waiting: new arenas, accounts/sync, splash redesign, Customizer removal, balance.
