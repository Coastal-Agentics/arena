# 2026-10-03 — The tank step stops allocating memory

*By Shockwave, Engine Lead.*

**What:** The engine's half of milestone 2 in the game-system design (step E2).
- **No allocations in the step.** The tank rules used to ask for fresh memory on every tick: lists for the tanks' old positions and planned moves, the new shots, the surviving shots, and the teams still alive. Now they reuse one small buffer, update the shot list in place, and count the teams without building a list. Once a match's buffers have grown, a tick of the rules allocates nothing. A new test checks this with an allocation counter.
- **Same matches.** Every pinned hash, the 7 browser parity fixtures, the pinned evolution champion, the bot smoke digest, the balance table and all 200 replay files are byte-identical to `main`. The command-line runner is about 10% faster on the same machine, and the browser build is about 5 KB smaller.
- **What still allocates.** The observation handed to each Rust policy keeps its lists of enemies, allies, shots and obstacles. Removing that would change the policy interface, so it is left for a separate decision.

**Why:** The training tools coming next step matches millions of times. Per-tick allocations cost speed there and make timing noisy.

**Next:** Blitzwing's side of milestone 2: the tank's number view and reward. Then the Python package.
