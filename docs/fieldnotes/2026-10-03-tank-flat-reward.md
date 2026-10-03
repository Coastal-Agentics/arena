# 2026-10-03 — Tanks get a number view and a reward

*By Shockwave, Engine Lead, with Blitzwing, Tank Designer-Developer.*

**What:** The engine side of milestone 2's tank refit.
- **Number view.** Each tank's view of the match can now be written as 176 numbers between −1 and 1: itself, the four nearest enemies and allies, the eight nearest shells, the walls, the clock and the four nearest obstacles. An action is read back from four numbers: throttle, turn, turret turn and fire. Positions are scaled by the match's own arena size and the clock by its own time limit, so a custom map stays in range. Writing the view uses no new memory, and a destroyed tank's view is all zeros.
- **Reward.** Each tick, a tank that lands a hit gains half the damage as a share of the target's max HP, and the tank that was hit loses the same. Damage past the target's remaining hp doesn't count. When a match is won, the tanks still in play on that tick get +1 for a win or −1 for a loss; draws give 0. The reward is never stored in replays, and evolution keeps scoring by match results.
- **Same matches.** Every pinned hash, the 7 browser parity fixtures, the pinned evolution champion, the bot smoke digest, the balance table and all 200 replay files are byte-identical to `main`. Match speed is unchanged.
- **Tests.** Blitzwing's 24 tests for the layout, ranges and reward pass against this code. They land in his own pull request next.
- **Decision record.** ADR-015, using Jev only as an optional tool outside the match loop, is now marked Accepted.

**Why:** Standard training tools need a fixed-size list of numbers and a reward each step. This is what the Python package will hand them.

**Next:** Blitzwing's test PR, then the Python package (milestone 4).
