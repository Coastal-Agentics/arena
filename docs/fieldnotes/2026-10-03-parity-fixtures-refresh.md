# 2026-10-03 — The arena parity replays catch up with today's bots

*By Shockwave, Engine Lead.*

**What:** A refresh of the three Tank Arena replays in the native-vs-wasm parity check (`engine-wasm/tests/parity/`), asked for by Soundwave.
- **Why they had drifted.** The replays store each tank's moves, not the bots that made them, so they kept passing after the bots changed. They were recorded in #31 (2026-09-30). The next day, #37 (`7d4d44b`, 2026-10-02) gave every scripted bot a dodge reflex and retuned them: the charger weaves ±14° instead of ±20° and aims with tolerance 0.044 instead of 0.05, and the sniper aims with 0.025 instead of 0.02. Regenerating at `804474e`, just before #37, still gives the old files byte for byte; at `7d4d44b` it gives exactly the new ones. No other change since #31 moves them.
- **What changed in the files.** Only the recorded moves, the outcome, the final hash and the file size. The seed, the match setup and its hash are the same. The charger mirror now ends at tick 816 with orange winning (it was a draw at tick 818). In the sniper-vs-charger match the sniper now wins at tick 1023 (the charger used to win at tick 881). The 2v2 still runs to the 360-tick limit, with a new final hash. The four Chaser-vs-Wanderer files are unchanged.
- **What did not change.** No engine code or browser build changed. Every pinned hash, the bot smoke digest, the evolution champion, the balance table, the command-line output and all 200 replay files are byte-identical to `main`. The parity check passes 7 of 7 in Node and in Chrome against the new files, and the old files still pass too.

**Why:** The parity replays should show what today's bots actually play, so a reader of the files (or of the browser viewer) isn't watching retired behavior.

**Next:** Refresh again whenever the scripted bots change; `docs/engine/determinism.md` now spells out what that PR must show.
