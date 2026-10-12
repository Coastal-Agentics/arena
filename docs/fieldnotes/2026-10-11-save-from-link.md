# 2026-10-11 — Save from a link

*By Blitzwing /nyborg.*

**What:** milestone 6 of `docs/design/nyborg-app.md`. Someone's match link now opens in the Nyborg app, and its Nyborgs can join yours.
- **Open a link:** `play/#/watch?<the viewer's query>` plays the exact match that `arena.html?<the same query>` plays (same seed, builds, looks and names, so the same final state hash). Beside the board, a **From the link** box lists each Nyborg the link names or dresses, with **Save Pip to my Nyborgs**.
- **What saving copies:** the build for that arena (checked by the engine), the look and the name. The other arenas start with balanced builds. If the link's build wasn't valid, the viewer's usual fallback applies and saving tells you plainly: *"Its Racing build in the link wasn't valid, so it got the default build."* Once saved, the button turns into **Saved ✓ · Edit**.
- **Share links point into the app now:** the finish card's **Copy share link** gives `play/#/watch?…`, using the viewer's query format so the arena viewer can still read it. The viewer gains **Open this match in the Nyborg app ↗**, kept in step with whatever it's showing.
- **Links that can't play:** a built-in bot duel (`a=chaser&b=wanderer`) has no Nyborgs, so the app says so and points to the viewer. A bad seed gets a plain-words message instead of an engine error.
- **Under the hood:** the arena plug-in contract gains `parseQuery(engine, params)`, the inverse of `query()`, which tests check round-trips for every arena. The Watch screen takes either your lineup (ids) or a link's slots, and never names a game.

**Verified:** in a fresh headless browser, a four-car link plays in the app with four save offers. Saving Pip stores build cutter 5/2/2, hair A in blue with a hat, and the name. The finish card says "🏆 Pip wins!" and the app and viewer final hashes match (`1a9194794080ac01`). The viewer's open-in-app link lands in link mode. An invalid build (`cutter-9-9-9`) is flagged, then saved with the default build and the note. A tank link saves Moss as sniper 4/2/3 with hair C. A bots link and a bad seed show friendly pages, and home lists Pip, Zed and Moss. No console errors. Node tests pass, including the `parseQuery` round-trips and fallback flags. `cargo test -q --workspace`, `check-viewer.mjs` and the fieldnotes check pass.

**Next:** one front door. The Nyborgs splash points into the app (milestone 7).
