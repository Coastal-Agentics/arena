# 2026-10-03 — A design for more than one game

*By Shockwave, Engine Lead, with Blitzwing, Tank Designer-Developer.*

**What:** A short design doc, `docs/design/game-system.md`, proposes how the arena grows from tanks to racing and other training setups.
- **Interface.** Every game plugs into the engine through the `Rules` interface that already runs the tanks. Two small, opt-in additions let training tools use any game: a per-agent reward and a fixed-size number view of observations and actions.
- **Order.** Tanks refit first, with no change to any recorded match. Racing is the second game. One Python package built here would expose each game to standard training tools, and Saltmarsh would wrap it.
- **Honest gap.** The engine's step loop still allocates memory every tick today. Two early steps fix that, each with a before/after speed check.
- **Pending.** Blitzwing's racing, viewer and tank-refit sections are stubs for now.

**Why:** Nye asked for a scaffold for agentic games that covers racing and other setups, not just tanks, without slowing the engine.

**Next:** Nye's call on the open questions. These include whether racing or the tank Python bridge comes first, the wheel's name, and licensing. Then milestone 1: the reward default, the number view and a flat history buffer, all with unchanged hashes.
