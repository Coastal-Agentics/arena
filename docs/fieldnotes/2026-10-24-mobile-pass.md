# 2026-10-24 — Mobile pass on the Nyborg app

*By Blitzwing /nyborg.*

**What:** a phone-width pass on `/arena/play/` so ~5 friends can hatch, set up a race, watch, and use the finish card on a ~390 px screen.
- Bigger touch targets on home, hatch, match setup, watch and the finish card (buttons ≥40–48 px, colour swatches 40 px).
- Less scroll on hatch (smaller stage); match setup no longer sideways-scrolls; Start is a full-width primary.
- Finish card actions (Again / New seed / Tweak / Change lineup / Copy share link) sit in a tappable 2-column grid with Again full-width.
- Bug fix on the way: each arena module had both a `start` label string and a `start(engine, match)` method; the method won, so setup rendered the whole function source as the CTA. The label is now `startLabel`; the method stays `start`.

**Verified:** headless Chrome at 390×844 — home, hatch, setup, watch and finish screenshots under `mobile-*.png`, no horizontal overflow, no console errors. Desktop grids (spotlight / hatch / slot) unchanged at 1180 px. `node --test tests/web/app.test.mjs` 11/11. Sim, pins and match hashes untouched; `.github/`, `engine*/`, `scripts/` untouched.

**Next:** share pack on the finish card (copyable link + guest "open this race" without a library; invite copy **Race my Nyborg**), after Soundwave looks at this pass.
