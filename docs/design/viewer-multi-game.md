# Viewer and Customize tab for more than one game

**Status:** Draft for review (2026-10-03).
**Author:** Blitzwing (Tank Designer-Developer). Part of [game-system.md](game-system.md) §4 (Shockwave).
**In one line:** one viewer page, a game picker, and one small module per game. Every tank link that works today keeps working, unchanged.

**Direction (naming and direction only; no code is renamed and nothing in scope changes):** Nye calls the agents **Nyborgs**, and one customized Nyborg can play in several arenas: a tank in Tank Arena and a car in Racing. The per-game schemas below are the first step. Customize can later grow into one per-Nyborg profile plus a loadout for each arena. Maps and tracks also become data that can be customized later; the tank arena and Ring are just the first ones.

## Today
- **Page and scripts:**
  - `web/arena.html` has two tabs, **Watch** and **Customize**.
  - `web/arena.js` (441 lines) runs everything: the play loop, the canvas drawing, the cards and the URL.
  - `web/tank-ui.js` holds tank helpers with no DOM: URL ↔ match, triangle geometry and readouts.
- **The wasm is tank-shaped:**
  - matches: `WasmMatch.tank(query)`, plus `WasmMatch.new`/`withConfig` for the Chaser/Wanderer bots;
  - Customize data: `tankCatalogJson`, `snapLoadout`, `canonicalTankQuery`;
  - parity: `checkReplayJson`.
- **URLs:** `?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`, the legacy `?seed=…&a=Chaser&b=Wanderer`, and the extras `tab`, `speed`, `paused` and `t`.
- **Training badge:** static "Scripted". The Gen badge and generation slider (GATE-003 M2 UI) are not built, and nothing in `web/` reads `web/data/` yet.

## The plan
**1. A game picker and a shell.** A "Game" selector above the tabs (Tank Arena · Racing). `arena.js` becomes the shell: tabs, play/pause, speed, seed, URL sync and the loop. Everything game-specific moves into a module, and the shell keeps a registry keyed by game id:

| Module | Gives the shell |
|---|---|
| `web/games/tank.js` (today's tank code from `arena.js` and `tank-ui.js`, moved, not rewritten) | `start(query)`, `draw(ctx, state)`, cards, result line, Customize schema, `canonicalQuery` |
| `web/games/racing.js` | The same set: draws the track, gates, cars with headings, and a lap and position board |

**2. One wasm package, one thin wrapper per game.** `WasmMatch` stays exactly as it is for tanks. Racing gets `WasmRace` with the same method names (`step`, `tick`, `isOver`, `stateJson`, `outcomeJson`, `stateHash`, `setupJson`), so the shell's loop doesn't care which game it is. All rules, tables and snapping stay in Rust and reach the page through the wasm, so the page can't drift from the sim. Only one `web/pkg` gets the CI drift check. Each PR reports the size increase.

**3. Customize from a per-game schema.** The page builds the tab from one JSON schema per game (`catalogJson(game)`; `tankCatalogJson` stays as is):

| Schema field | Tank Arena | Racing |
|---|---|---|
| Slots | Blue, Orange | Car 1–4 (2–4 in the race) |
| Behaviors (key, name, training) | Charger, Kiter, Sniper | Follower, Cutter, Blocker |
| Stats, budget | Attack / Speed / Defense, 9 points, 1–5 each (19 loadouts) | Power / Top speed / Grip, 9 points, 1–5 each (19 setups) |
| Presets | Balanced 3/3/3, Glass Cannon 5/3/1, Brawler 4/1/4, Scout 2/5/2 | Balanced 3/3/3, plus presets set in the racing BALANCE.md |
| Readout per slot | damage, reload, max speed, HP, hits to kill | acceleration, top speed, grip, solo lap estimate |
| Match settings | seed | seed, laps, track |

The triangle widget is already generic over three stats, so both games reuse it with the same snapping rule.

**4. URLs.** A `game` parameter is added:
- **No `game` means Tank Arena.** The canonical tank link never prints `game=tank`, so every existing link and the Customize "Link" stay byte-identical.
- **Racing:** `?game=racing&seed=42&track=ring&laps=3&cars=cutter-3-3-3,follower-4-2-3,blocker-2-3-4`. Each car is a behavior plus its Power-Top speed-Grip setup, written like a tank loadout.
- **Bad values:** an unknown game, or a bad value, shows the same inline error the tank page shows today.
- **Extras:** `tab`, `speed`, `paused` and `t` work for every game.

**5. Replays.** When the viewer loads replay files, it reads **format 4** (no `game` field, so Tank Arena) and **format 5** (it picks the module from `game`). It refuses a `rules_version` it doesn't know, with a plain message, instead of playing the replay wrong.

**6. Training indicator and generation slider, shared.**
- **Built once in the shell:** the Gen badge and slider from GATE-003 M2 work the same way for every game.
- **Data:** each game's evolution files live in `web/data/<game>/evolution/` (`champion.json`, `history.json`, …).
- **Badge:** reads "Scripted" or "Gen N · win rate vs Gen 0 · experimental/promotable".
- **Slider:** picks the generation shown in the behavior list.
- **Tank's data path:** tank's files are at `web/data/evolution/` on `nightly-data`, so moving them to `web/data/tank/evolution/` changes the nightly's output path. That is a CoS change in one coordinated `nightly-data` → `main` PR (ADR-007). Until then, the shell maps tank to the current path.

## Milestones and acceptance (game side)
| | What | Accepted when |
|---|---|---|
| **V1** (tank only; can land any time before M5) | Shell + registry; tank code moved into `web/games/tank.js` | No visible change. These pass unchanged: `check-viewer.mjs` (16,923 checks today), `check-parity.mjs` (7 fixtures) and `check-viewer-browser.py`. Every old URL gives the same canonical link and the same match hashes. |
| **V2** (= racing R3) | `WasmRace`, `racing.js` Watch | A racing link plays the same ticks and final hash as native. Racing parity fixtures are in CI. |
| **V3** | Customize from the schema, for both games | Racing URLs round-trip. The triangle can't make an invalid setup. Tank behaves as in V1. |
| **V4** | Shared Gen badge and slider (the GATE-003 M2 UI) | Works for tank from `nightly-data` data. Racing shows "Scripted" until it has a lineage. |

Viewer checks live in `scripts/`, and the CoS and Shockwave own that folder. V2 and V3 need racing cases added to those checks, done by whoever owns each script.
