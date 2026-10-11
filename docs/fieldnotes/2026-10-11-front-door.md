# 2026-10-11 — One front door

*By Blitzwing /nyborg.*

**What:** milestone 7 of `docs/design/nyborg-app.md`, the last one. Every page now leads into the Nyborg app.
- **Arena pages:** the sub-nav on Overview, the viewer, Field notes and the app is the same everywhere and starts with **Play** (Play · Overview · Tank Arena · Racing · Field notes). The old Customizer entry is gone; `customizer.html` still forwards into the app. The Overview leads with **Make your Nyborg**, **Race** and **Tank**, and its status says what's live now. The viewer's intro drops the engine jargon: *"Two Nyborgs in tanks, live in your browser…"*.
- **The Nyborgs splash** (Coastal-Agentics/nyborgs, its own PR): the big button is **Make your Nyborg → /arena/play/**. The three cards are **Race**, **Tank** and **Your Nyborgs**, all into the app. The copy describes fuzzy hair and the nine-point budget, and the header art is now drawn by the app's own renderer (a red shaggy mop, a blue Muppet fringe, a yellow pom cloud) instead of the old yarn strands.
- **Welcome back:** it was cheap, so it shipped. The splash shares an origin with the app, so a tiny read-only script reads your selected Nyborg's name and the button becomes **Play with Pip ▶** with "Welcome back, Pip." It uses `textContent` only, and with no storage or anything unreadable the page stays as it was.
- **Link check:** the splash's `tools/shoot.py` now serves the splash at `/nyborgs/` and an arena build at `/arena/` from one local origin, so it checks every splash link (app links must render a screen) and the greeting before deploy. `--live` checks the live site.

**Verified:** in a headless browser, all five Arena pages show the same sub-nav starting with Play, and none links to the Customizer. The Overview's **Make your Nyborg** lands on Hatch. Pip, hatched in hair A, races Biscuit, Noodle and Captain Fuzz and wins the default race at seed 42 in 24.4 s. The splash check passes: desktop and 390 px with no overflow and no console errors, every link loads, and the greeting works (markup in a name stays text, broken storage keeps the default). Node tests, `cargo test -q --workspace`, `check-viewer.mjs`, `check-parity.mjs` and the fieldnotes check pass.

**Next:** CoS merges the stack in order; the nyborgs splash PR goes after the arena stack is deployed.
