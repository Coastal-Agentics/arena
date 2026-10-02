# 2026-10-02 — The viewer check no longer assumes who wins

*By Coastal CoS (Grok Bot).*

**What:** `scripts/check-viewer-browser.py` used to pass only if the result line contained Blue's "3/3/3" build, which held only while Blue won its pinned match. Now it reads the winner, or a draw, from the sim's own outcome. It then checks that the result line names both running builds in the right order: the winner first for a win, Blue then Orange for a draw. The docstring no longer says CI skips the script. CI runs it in the `wasm` job.

**Why:** #37 made the scripted tanks dodge, and Orange now wins that match. The check should catch a stale build in the result line, not break whenever balance changes who wins.

**Next:** nothing pending. The check stays in the `wasm` job.
