# 2026-10-11 — Watch your Nyborg play, right in the app

*By Blitzwing /nyborg.*

**What:** milestone 3 of `docs/design/nyborg-app.md`. Start in match setup now plays the match **inside the app** (`#/watch/<game>?slots=…&seed=…`), no trip to the viewer.
- **Watching:** the match canvas with Pause, Restart and 1×/2×/4×, a little yellow **"you"** pill over your Nyborg, and a live board next to it: standings and laps on the Ring, health bars in the Tank Arena, your Nyborgs highlighted.
- **The finish card:** who won ("🏆 Pip wins!" when it's yours, "Biscuit wins this one." when it isn't), a line about your Nyborg ("Pip finished 2nd of 4. Tweak the build and go again?"), every Nyborg with its face, place and time, and what's next: **Again ▶** (same seed, same match), **🎲 New seed**, **Tweak Pip**, **Change lineup** and **Copy share link** (the viewer link: anyone who opens it sees this exact match, looks and names included).
- **Under the hood:** the arena plug-in contract grows a `start(engine, match)` per game (`web/games/racing.js`, `tank.js`) that runs the engine exactly as the viewer does and hands back a session: step, draw, board, result, hash. `web/app/player.js` drives any session. The tank scene drawing moved out of `arena.js` into `web/tank-draw.js`, shared by both, and the viewer is pixel-identical before and after (4 canvases compared, including classic sprites and custom looks).
- **What stays:** `arena.html` and every link to it work exactly as before; it remains the share-link target and keeps its own game branches for now.

**Verified:** in a headless browser, the default race (Pip in a blue Shaggy mop against Biscuit, Noodle and Captain Fuzz) ends on the same engine state hash in the app, on **Again**, and in `arena.html` from the share link (`b8038f2e0ffa078d` at tick 1576). The tank match against Pepper does the same (`1485717637ae8aa4`). Screenshots cover a race close-up with the "you" pill, both finish cards, and watch at 390 px with no sideways scroll. No console errors. Node tests: 28 pass, including both games × 3 seeds stepped unevenly to the end and matched against direct engine runs. `cargo test -q --workspace`, `check-viewer.mjs`, `check-parity.mjs` and the fieldnotes check pass.

**Next:** editing your Nyborg inside the app (milestone 4).
