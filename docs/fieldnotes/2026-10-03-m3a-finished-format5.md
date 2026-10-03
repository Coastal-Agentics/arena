# 2026-10-03 — The engine gets ready for a second game

*By Shockwave, Engine Lead.*

**What:** The engine side of milestone 3 in the game-system design (M3a), ahead of racing, which Nye approved today. No racing code yet: Blitzwing's racing rules come next.
- **A "finished" ending.** Matches could end three ways: one team left standing, everyone destroyed, or the time limit. A game can now also say "finished", meaning its own goal was reached, such as every car crossing the line. Tank Arena never uses it.
- **Replays name their game.** Replay format 5 adds two fields: which game the file is for, and which version of that game's rules. Loading a file checks both first, so a racing replay opened as a tank match fails with a plain message ("replay is for game \"racing\"") instead of a confusing error, and a replay from older rules is refused instead of playing out wrong.
- **Tank Arena is untouched.** Tank keeps writing its format 4 files byte for byte. Every pinned hash, the 200 replay files, the command-line output, the balance tables, the 7 browser parity fixtures and the pinned evolution champion are identical to `main`. The 7 committed format 4 fixtures now also have a test that loads, verifies and writes each one back byte for byte.
- **Still fast.** Nothing changed in the per-tick loop. Speed is the same within measurement noise, and the browser build is about 5.7 KB (1.7%) larger, from the new replay checks.

**Why:** Racing needs its own ending and its own replay files, and the viewer needs to know which game a replay belongs to.

**Next:** Blitzwing's racing rules (R1), which plug into these. Then the racing viewer.
