# 2026-10-03 — Milestone 1: the shared game interface, with unchanged matches

*By Shockwave, Engine Lead.*

**What:** The first milestone of the game-system design (`docs/design/game-system.md`) is in the engine.
- **Reward hook.** Every game's rules now have a per-agent reward that is 0.0 unless the game defines one. It is computed from the match state and that step's events, and it is never stored in a replay.
- **Number view.** An opt-in, fixed-size view turns one agent's observation into a list of numbers written into a buffer the caller owns, and turns a list of numbers back into an action. This is what the Python package will use later. Tanks don't use it yet; the tank encoding is milestone 2, with Blitzwing.
- **Flat history.** A match used to keep one small list of actions per tick. It now keeps every action in one buffer and reuses one scratch list, which removes two memory allocations per tick.
- **Same matches.** Every pinned hash, the 7 browser parity fixtures, the pinned evolution champion, the bot smoke digest and all 200 replay files are byte-identical to `main`. The command-line runner is about 18% faster on the same machine.
- **Nyborgs.** The customizable agents now have a name: Nyborgs. One Nyborg will be able to play in several arenas. The interface keeps what an agent is apart from what the map is, so both can become customizable.

**Why:** Nye approved the design and the order: this interface first, then the tank refit, then the Python package, then racing.

**Next:** Milestone 2, the tank refit. Blitzwing defines the tank's number view and reward. The engine side removes the tank step's remaining per-tick allocations.
