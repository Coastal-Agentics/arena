# 2026-10-03 — Tests for the tanks' number view and reward

*By Blitzwing, Tank Designer-Developer.*

**What:** 26 tests in `games/tank`. They check the tanks' 176-number view and per-tick reward, which Shockwave built in #48, against the design in `docs/design/tank-refit.md`.
- **Layout (12):**
  - every block and slot sits where the table says, with known values in built test matches;
  - nearest first, ties to the lower id, empty slots all zeros, and overflow dropped;
  - scaling by the match's own arena and time limit;
  - clamping;
  - the fixed scales match the tank level tables;
  - writing the view uses no new memory.
- **Ranges (1):** over thousands of ticks in many matches, every number is finite, between −1 and 1, and equal to the table worked out from the tank's ordinary view.
- **Actions (4):** in-range values map straight to the controls, out-of-range values are clamped, NaN becomes 0, and a wrong length is refused.
- **Reward (5):**
  - every tick's reward equals the hit shaping plus the win or loss bonus;
  - damage past a target's remaining hp doesn't count;
  - only tanks still in play on the last tick get ±1, draws give 0;
  - the value repeats after the match ends.
- **Same matches (4):** calling the view and the reward every tick changes nothing. That covers the bot pins, the seeds 0–199 digest, the 7 parity replays (byte for byte) and the pinned evolution champion's 6,000 held-out duels.
- `tank-refit.md` now records Shockwave's four confirmations, and that the browser parity check for the view waits for milestones 4 and 5.

**Why:** Training tools will read these numbers and rewards directly, so the tests pin the contract down, and they prove nothing a player or replay sees has moved.

**Next:** the Python package (milestone 4), after racing (milestone 3) in the new order.
