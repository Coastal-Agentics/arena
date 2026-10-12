# 2026-10-11 — New hair: pick your Nyborg's style

*By Blitzwing /nyborg.*

**What:** Nye picked on #85: players choose from several styles. Every Nyborg now has a hair style: **Shaggy mop** (the default), **Pom cloud**, **Muppet fringe**, **Tousled crest**, or **Classic yarn** (the old strands, still there for anyone who misses them).
- **Drawing:** `web/nyborg-hair.js` holds the four fuzzy styles. `drawNyborg()` takes `style` and `id`, and the id seeds the fur's jitter (no `Math.random`). Yarn is drawn by exactly the old code. The viewer (racing cars, tank Nyborgs) and the Customizer (library cards, big preview, match-pick faces) all draw the chosen style. Nyborgs with no look of their own (today's default grid and tank teams) wear the mop in their team colours.
- **Save format 2:** the look gains `hair_style`. On first load, a format-1 library is copied whole to `arena.nyborg.backup.v1` *first*, then every Nyborg is upgraded to the Shaggy mop (keeping its strand count for yarn). If the backup can't be written, nothing is written and the Customizer asks for an export. A friendly banner says what happened. Format-1 files still import (upgraded, with a note). Old cached pages show format-2 Nyborgs read-only, as before.
- **Links:** the look token's second field is now a style letter (`m`, `p`, `f`, `c`) or, for Classic yarn, the strand count, so `looks=3F6FD8-m-h` is a blue mop with a hat. Every link shared before today still reads as the same yarn look. Links carry no Nyborg id, so the viewer seeds the fur from the Nyborg's name (or its slot).
- **Customizer:** the Look panel gets a Hair style row: five buttons, each showing *your* Nyborg in that style. The Strands row appears only for Classic yarn.

**Verified:** headless screenshots of a seeded format-1 library migrating (banner, backup written first, Pip's look upgraded with strands kept), the style picker, a race and a tank match from Race these / Watch links with Pip (blue mop, hat) and Juno (yellow pom, glasses), the default race and tank, and the regenerated style sheet. No console errors. Node tests (`customizer`, `hair`, `race-these`) pass 21, including three new migration and shape tests and the style link round trips. `cargo test -q --workspace` keeps the pins. `check-viewer.mjs`, `check-parity.mjs` and the fieldnotes check pass. No change to `engine*/`, `scripts/`, `.github/` or `web/pkg`.

**Next:** the Nyborg app's home at `/arena/play/` (milestone 1).
