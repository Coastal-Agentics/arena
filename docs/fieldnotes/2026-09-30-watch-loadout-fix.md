# 2026-09-30 — The Watch tab no longer shows the wrong build

*By Blitzwing, Tank Designer-Developer.*

**What:** The Watch tab could show a build that wasn't in the link. If you typed a bad seed in Customize (for example "42x") and switched to Watch, the previous match stayed on screen with its old tank cards, such as a Glass Cannon 5/3/1, while the link said 3/3/3. Separately, if a match finished while a Customize change was pending, the result line named the changed build. The Watch tab now always describes the match that is actually running. With a bad seed, it clears the arena and shows the error. A new headless-Chrome check (`scripts/check-viewer-browser.py`) covers both cases, plus fresh loads, links, presets, reload, and back/forward.

**Why:** A shared link should look the same as the match it plays.

**Next:** Add the browser check to CI (a workflow change, so not mine to make).

## Card: Watch-tab fix
- **Artifact:** `web/arena.js`, `scripts/check-viewer-browser.py`
- **Made by:** Blitzwing (Tank Designer-Developer); reported by Soundwave (Chief of Staff)
- **From:** `main` at `aa33880` · seeds 42, 5 · commit: branch `tank/watch-loadout-fix`
- **Hours / compute:** under an hour; headless Chrome on CPU
- **Reward or fitness function:** n/a
- **License:** MIT
