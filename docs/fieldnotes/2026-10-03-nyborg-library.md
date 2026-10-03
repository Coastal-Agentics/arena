# 2026-10-03 — A library of Nyborgs (design for a gate)

*By Blitzwing (Tank Designer-Developer).*

**What:** A short design, `docs/design/nyborg-library.md`, and a wireframe, `docs/design/nyborg-library-wireframe.svg` and `.png`, for keeping many Nyborgs and customizing them in one place.
- **Save format:** each Nyborg is a small versioned JSON file. It holds a name, a look (hair color, 2–4 strands, accessories) and, for each game, a build of levels with a behavior, either scripted or an evolved champion.
- **Storage:** the library lives in your browser, and you can export or import one Nyborg or the whole library as a file. There is no server.
- **One Customizer:** a library list, plus one tab per game. Each tab is built from the engine's catalog for that game, so Tank Arena and Racing plug in the same way, and so will later games.
- **Equal footing:** every Nyborg plays on the same 9-point budget. The same engine check runs in the Customizer, on import and at match start. Looks never reach the sim, replays or hashes.

**Why:** Nye's vision is many Nyborgs, all on equal footing, playing tanks and racing. Today's tank-only Customize tab can grow into that one Customizer without changing a single tank link, replay or pin.

**Next:** Nye's call on the open questions. Then Shockwave's catalog and validator in the wasm, followed by the save format (N4) and the one Customizer for tank (N5). Racing joins in N6, once racing lands.
