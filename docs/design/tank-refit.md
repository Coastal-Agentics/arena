# Tank refit to the shared interface: constraints

**Status:** Stub. Pending: Blitzwing (Tank Designer-Developer) writes this section.

Planned contents, from [game-system.md](game-system.md) §2:
- what the refit adds (scratch buffers in `TankRules`, `tank::encode_obs` as the `Flat` view, a tank reward) and what it must not change;
- the bot and M1 champion hash pins, unchanged;
- the parity replays and their hashes, byte-identical (replay format stays 4);
- how each refit PR proves it (parity CI, pinned tests).
