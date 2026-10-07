# engine-wasm and the web viewer

Source: `engine-wasm/` (`Cargo.toml`, `src/lib.rs`), `scripts/build-wasm.sh`,
`rust-toolchain.toml`, `web/arena.html`, `web/arena.js`, `web/tank-ui.js`, `web/pkg/`,
`games/tank/src/matchup.rs` and `games/tank/src/loadout.rs` (the Tank Arena API),
`.github/workflows/ci.yml` (`wasm` job) and `.github/workflows/pages.yml`.

The viewer landed on `main` in PR #8. It plays **live** matches: between the placeholder bots
(`tank::Chaser`, `tank::Wanderer`), or Tank Arena duels (rules v1, #18). It doesn't load
replay files. Racing has a live step-and-state API too, `WasmRace` ([below](#racing-wasmrace-m5)),
for the race viewer.

## How the pieces plug together

```
engine (rlib) ──► games/tank (rlib: bots, rules v1)
      │                  │
      └──────────────────┴──►  engine-wasm (cdylib + rlib, wasm-bindgen =0.2.100)
                                  │  scripts/build-wasm.sh
                                  ▼
                    web/pkg/engine_wasm.js       (ES module glue, wasm-bindgen --target web)
                    web/pkg/engine_wasm_bg.wasm  (the compiled engine)
                        ▲  import init, { WasmMatch } from "./pkg/engine_wasm.js"
                    web/arena.js  ◄── <script type="module" src="./arena.js"> in web/arena.html
```

`engine` itself has no wasm-specific code. It builds for `wasm32-unknown-unknown` because it
avoids OS, thread and time dependencies, and pulls in `rand_chacha` without default features
(no `getrandom`). CI checks this with `cargo build -p engine --target wasm32-unknown-unknown`.

## engine-wasm

Two layers:

- **`Viewer`**, plain Rust and unit-tested natively. It wraps a `Match` and two boxed
  policies. For the placeholder bots (`tank::Chaser`, `tank::Wanderer`) they're chosen by name with `BotKind::parse` (`"chaser"` or `"wanderer"`,
  case-insensitive, trimmed). `Viewer::new(seed, team0, team1)` uses `MatchConfig::duel()`;
  `Viewer::with_config(config, seed, team0, team1)` takes any `MatchConfig`. Either way the
  first bot drives tank 0 and the second tank 1; any further tanks idle.
  `Viewer::tank(query)` builds a Tank Arena duel from `tank::MatchSpec::from_query` and keeps
  its `SetupView` (`Viewer::setup()`).
  - `Viewer::step(n)` runs up to `n` ticks with the same logic as `Match::step_policies` (dead
    tanks idle) and stops early at the end.
  - `Viewer::state()` returns a `StateView` for drawing.
- **`WasmMatch`**, the `#[wasm_bindgen]` wrapper. It only forwards calls and turns `String`
  errors into JS `Error`s.

Wanderer seeding: `seed ^ 0x5eed` on team 1 (so Chaser vs Wanderer equals
`engine-cli --seed <seed>`; test `chaser_vs_wanderer_matches_headless_run`), and
`seed ^ 0x5eed_0000` on team 0.

### JS API (from `web/pkg/engine_wasm.js`)

| JS | Returns | Notes |
| --- | --- | --- |
| `await init()` (default export) | | Loads `engine_wasm_bg.wasm` from `new URL('engine_wasm_bg.wasm', import.meta.url)`, i.e. next to the JS file |
| `initSync({ module })` | | Same, from bytes you already have (e.g. `readFileSync` in Node; `scripts/check-viewer.mjs` does this) |
| `new WasmMatch(seed, team0, team1)` | `WasmMatch` | `MatchConfig::duel()`. `seed` is a **decimal string**; bots `"Chaser"`/`"Wanderer"`. Throws on bad input |
| `WasmMatch.withConfig(configJson, seed, team0, team1)` | `WasmMatch` | Same, with a custom `MatchConfig` as a JSON string ([config](replay-format.md#config)), e.g. per-tank params. A tank's own `params` must be a valid build on the shared params (see [Builds](#builds-the-per-game-catalog)). Throws `config json: …` on bad JSON or a missing field, and `tanks[i].params is not a valid 9-point build on the shared params` otherwise |
| `m.step(n)` | `boolean` | Advance up to `n` ticks; `true` once the match is over |
| `m.tick()` | `number` | Ticks so far |
| `m.isOver()` | `boolean` | |
| `m.stateJson()` | `string` | JSON `StateView`, below |
| `m.outcomeJson()` | `string` | `"null"` while running, else `{"winner":…,"ticks":…,"reason":…}` |
| `m.stateHash()` | `string` | 16 hex digits; at the end of Chaser vs Wanderer, equal to `engine-cli`'s `hash` |
| `m.setupJson()` | `string` | Tank Arena duels: JSON `SetupView` (below); `"null"` for a placeholder-bot duel |
| `m.free()` | | Release the Rust object |
| `duelConfigJson()` | `string` | `MatchConfig::duel()` as JSON, a starting point for `withConfig` |
| `WasmMatch.tank(query)` | `WasmMatch` | A Tank Arena duel from a URL query (see below). Throws on bad input |
| `WasmMatch.fromBuilds(game, seed, blueBuildJson, orangeBuildJson)` | `WasmMatch` | A Tank Arena duel from two builds, each checked by `validateBuild`; plays exactly like `WasmMatch.tank` with the same seed, behaviors and levels. Throws `blue: invalid build [{"code":…,"key":…}]` (or `orange: …`), and for a champion behavior (the loader resolves those). Tank only: any other game, racing included, throws `wrong_game` |
| `games()` | `string` | JSON `[{"game": "tank", "rules_version": 1}, {"game": "racing", "rules_version": 1}]` |
| `catalogJson(game)` | `string` | JSON build catalog for a game (below). Throws `unknown game "…"` |
| `defaultBuild(game)` | `string` | JSON default build (3/3/3, first scripted behavior: tank `charger`, racing `follower`). Throws for an unknown game |
| `validateBuild(game, buildJson)` | `string` | JSON `{"ok": true, …}` with the normalized build, or `{"ok": false, "errors": […]}`. Never throws |
| `tankCatalogJson()` | `string` | JSON `CatalogView`: the Customize tab's tables and lists (below) |
| `snapLoadout(attack, speed, defense)` | `string` | Snap barycentric weights (Attack, Speed and Defense corners of the Customize triangle) to the nearest valid loadout, `"A-S-D"` (`tank::Loadout::snap`) |
| `canonicalTankQuery(query)` | `string` | Canonical `seed=…&blue=…&orange=…` for a query; throws on invalid input |
| `checkReplayJson(json)` | `string` | Re-simulate a replay file (any readable format) and return JSON `{format, seed, tanks, ticks, outcome, final_hash, setup_hash, verify_error}`. Everything but `format` and `seed` is recomputed, not copied from the file; `verify_error` is `null` if `Replay::verify` passes, else its message. Throws if the JSON doesn't load as a replay. Used by the [parity check](determinism.md#native-vs-wasm-parity) |
| `engineVersion()` | `string` | `engine` crate version |

`withConfig(duelConfigJson(), seed, a, b)` plays exactly like `new WasmMatch(seed, a, b)`
(test `custom_config_with_per_tank_params`; checked in headless Chrome for six seeds). A
loadout from JS: a spawn's params replace the shared set as a whole, and must be exactly a
valid build applied to the shared params (`tank::Loadout::apply`: Attack sets
`projectile_damage`, Speed `max_speed`, `turn_rate` and `fire_cooldown`, Defense `max_hp`),
or the shared params unchanged. Anything else throws, so a config can't hand a tank
more than 9 points:

```js
const cfg = JSON.parse(duelConfigJson());
const c = JSON.parse(catalogJson("tank"));
const lv = (stat, level) => c.stats.find((s) => s.key === stat).values[level - 1];
// Glass Cannon 5/3/1 for tank 0 and Brawler 4/1/4 for tank 1.
cfg.tanks[0].params = { ...cfg.params, projectile_damage: lv("attack", 5).damage, ...lv("speed", 3), max_hp: lv("defense", 1).max_hp };
cfg.tanks[1].params = { ...cfg.params, projectile_damage: lv("attack", 4).damage, ...lv("speed", 1), max_hp: lv("defense", 4).max_hp };
for (const t of cfg.tanks) delete t.params.level;
const m = WasmMatch.withConfig(JSON.stringify(cfg), "42", "Chaser", "Wanderer");
```

### Builds: the per-game catalog

A **build** is the per-game part of a Nyborg ([nyborg-library.md](../design/nyborg-library.md)):
levels plus a behavior, with no resolved numbers. The shape and the validator are shared
by every game, in the small `game-catalog` crate (`games/catalog`, outside `engine/`); each
game crate fills it in from its own tables (`tank::catalog`, from `tank::loadout`). The
engine never sees a Nyborg's `look`.

```json
{"rules_version": 1,
 "levels": {"attack": 5, "speed": 3, "defense": 1},
 "behavior": {"kind": "scripted", "id": "kiter"}}
```

`behavior` may instead be `{"kind": "champion", "ref": "tank/nightly/gen-99"}`; only its
shape is checked (a non-empty string of at most 200 bytes), because the loader resolves the
ref. Unknown top-level fields (a `look`, a `game: "tank"`) are ignored.

- **`games()`**: `[{"game":"tank","rules_version":1},{"game":"racing","rules_version":1}]`.
- **Racing** (since 2026-10-03): `catalogJson("racing")` is the committed
  `games/racing/catalog.json` (budget 9; stats `power`, `top_speed`, `grip`, levels 1–5 at
  one point each, resolving to `acceleration`, `top_speed` and `grip`; behaviors
  `follower`, `cutter`, `blocker`; presets Balanced, Sprinter, Speedster, Carver).
  `defaultBuild("racing")` is 3/3/3 `follower`. `validateBuild("racing", …)` uses the same
  error codes and order as tank, with the car's params on success:
  `{"ok":true,"game":"racing","rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"scripted","id":"follower"},"points":9,"params":{"power":240.0,"top_speed":240.0,"grip":0.25}}`.
  There is no race viewer yet (`WasmRace`), so a race can't be started from JS.
  `scripts/catalog_racing.test.mjs` (`node --test`) and engine-wasm's
  `build_exports_wrap_the_racing_catalog` cover every call and error code.
- **`catalogJson("tank")`**: `game`, `rules_version`, `budget` (9), `stats` (Attack, Speed,
  Defense, each `{key, label, min: 1, max: 5, cost_per_level: 1, values}` where `values[i]` is
  level `i + 1` resolved: `damage`; `max_speed`, `turn_rate` and `fire_cooldown` (reload
  ticks); `max_hp`), `behaviors` (`["charger","kiter","sniper"]`), `presets`
  (`{id, label, levels}`: Balanced 3/3/3, Glass Cannon 5/3/1, Brawler 4/1/4, Scout 2/5/2) and
  `default_build`. Abridged:

  ```json
  {"game":"tank","rules_version":1,"budget":9,
   "stats":[{"key":"attack","label":"Attack","min":1,"max":5,"cost_per_level":1,
             "values":[{"level":1,"damage":14},{"level":2,"damage":17},"…",{"level":5,"damage":29}]},
            {"key":"speed","label":"Speed","min":1,"max":5,"cost_per_level":1,
             "values":[{"level":1,"max_speed":90.0,"turn_rate":273,"fire_cooldown":64},"…"]},
            {"key":"defense","label":"Defense","min":1,"max":5,"cost_per_level":1,
             "values":[{"level":1,"max_hp":460},"…",{"level":5,"max_hp":940}]}],
   "behaviors":["charger","kiter","sniper"],
   "presets":[{"id":"balanced","label":"Balanced","levels":{"attack":3,"speed":3,"defense":3}},
              {"id":"glass_cannon","label":"Glass Cannon","levels":{"attack":5,"speed":3,"defense":1}},
              {"id":"brawler","label":"Brawler","levels":{"attack":4,"speed":1,"defense":4}},
              {"id":"scout","label":"Scout","levels":{"attack":2,"speed":5,"defense":2}}],
   "default_build":{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},
                    "behavior":{"kind":"scripted","id":"charger"}}}
  ```
- **`defaultBuild("tank")`**: the catalog's `default_build`.
- **`validateBuild(game, buildJson)`**: each level must be an integer 1 to 5 and the levels
  must spend exactly 9 points, which leaves 19 valid builds (tested). Success:

  ```json
  {"ok":true,"game":"tank","rules_version":1,
   "levels":{"attack":5,"speed":3,"defense":1},"behavior":{"kind":"scripted","id":"kiter"},
   "points":9,
   "params":{"radius":16.0,"max_speed":120.0,"turn_rate":364,"turret_turn_rate":546,"max_hp":460,
             "fire_cooldown":45,"projectile_speed":360.0,"projectile_ttl":120,
             "projectile_damage":29,"projectile_spread":256}}
  ```

  `params` is what the tank plays with (`Loadout::params`, the duel's shared params with the
  levels applied). Failure lists every error, each naming its key:

  ```json
  {"ok":false,"errors":[{"code":"unknown_key","key":"luck"},
                        {"code":"out_of_range","key":"attack"},
                        {"code":"unknown_behavior","key":"behavior"}]}
  ```

  | `code` | `key` | When |
  | --- | --- | --- |
  | `invalid_json` | `""` | The text doesn't parse, or isn't an object |
  | `wrong_game` | `game` | `game` isn't a known game, or the build's own `game` field differs |
  | `rules_version_mismatch` | `rules_version` | Missing, or not this game's `rules_version` |
  | `unknown_key` | the key | A `levels` key that isn't a stat |
  | `out_of_range` | the stat | Missing, not an integer, or outside 1 to 5 |
  | `over_budget` / `under_budget` | `levels` | The levels spend more / fewer than 9 points (only checked once every level is in range) |
  | `unknown_behavior` | `behavior` | Not a known scripted id (ids are lower case), or a champion without a usable `ref`, or another `kind` |

  The first three stop the check, since the levels mean nothing without them; the rest are
  all reported together.

**Adding a game** (racing did, 2026-10-03). The game crate provides plain data and a few functions,
no traits:

```rust
pub const GAME: &str = "racing";
pub const RULES_VERSION: u64 = 1;
/// Stat keys, ranges, costs, budget and behavior ids: what builds are checked against.
pub const RULES: game_catalog::Rules = /* ... */;
/// From the game's own level tables (stats power/top_speed/grip; behaviors
/// follower/cutter/blocker), so no number is written twice. Not called in wasm.
pub fn catalog() -> game_catalog::Catalog;
/// catalog() and its default build as JSON, committed (games/racing/catalog.json) so wasm
/// carries strings, not a JSON writer; a test keeps them equal and checks
/// catalog().matches(&RULES).
pub const CATALOG_JSON: &str = include_str!("../catalog.json");
pub const DEFAULT_BUILD_JSON: &str = r#"{...}"#;
/// game_catalog::validate(&RULES, json)?, then the game's resolved params.
pub fn validate_build(json: &str) -> Result<game_catalog::Valid<RaceParams>, Vec<game_catalog::BuildError>>;
```

`engine-wasm` then gets one entry in its `GAMES` list (id, rules version, the two JSON
strings, `validate_build`) plus the crate dependency; `games()`, `catalogJson`,
`defaultBuild` and `validateBuild` need no other change. Tank's catalog is regenerated with
`cargo run -q -p tank --example catalog_json > games/tank/catalog.json` (a test fails while it
is stale). Starting a race from builds (`WasmRace.fromBuilds`) comes with `WasmRace`.

**Enforcement.** Every JS path that starts a match obeys the same rule,
`game_catalog::check_levels` on `tank::catalog::RULES`: `fromBuilds` validates each build
with it (`tank::catalog::validate_build`); `WasmMatch.tank` and `withConfig` go through
`tank::Loadout`, which accepts exactly those levels (a test checks every level from 0 to 6
on each stat), and `withConfig` rejects per-tank params that aren't one of those loadouts
applied to the shared params (`check_config`). An imported or edited profile can't beat
the budget. Replays aren't builds: `checkReplayJson` still re-simulates whatever config a
replay file records.

### Tank Arena duels (rules v1)

Rules v1 (#18) added the Tank Arena API above: `WasmMatch.tank`, `setupJson`,
`tankCatalogJson`, `snapLoadout` and `canonicalTankQuery`. They wrap `games/tank`:

- **Query format** (`tank::MatchSpec::from_query`): `seed=<u64>&blue=<tank>&orange=<tank>`,
  with or without a leading `?`. A tank is `<behavior>-<attack>-<speed>-<defense>`
  (`kiter-5-3-1`); a bare behavior (`kiter`) means 3-3-3, and behavior names are
  case-insensitive. Behaviors are `charger`, `kiter` and `sniper`. A loadout must be 9
  points with each stat 1 to 5 (19 loadouts). Missing keys take the defaults
  (`seed=42&blue=kiter-3-3-3&orange=charger-3-3-3`). Unknown keys (the viewer's `tab`,
  `speed`, `t`, `paused`) are ignored. Bad input throws; the message comes from
  `games/tank`, e.g. `blue: stats sum to 10, must be exactly 9`.
- **The match** (`MatchSpec::start`): blue is tank 0 (team 0) and orange tank 1 (team 1), on
  the Tank Arena duel config (`tank::rules::duel`: 800×600 with two pillars, 7,200-tick
  limit, each tank's loadout as its per-tank params). Each policy's jitter RNG is seeded
  with the match seed XOR a per-side salt (`tank::matchup::BLUE_SALT`, `ORANGE_SALT`). `step`, `stateJson`,
  `stateHash` and the rest work as for the placeholder bots. The final hash equals native
  `tank::MatchSpec::from_query(q)?.run()`.
- **`SetupView`** (`m.setupJson()`): `{"query", "seed", "tanks": [{"behavior", "name",
  "training", "loadout"}, …]}`. `query` is the canonical query that reproduces the match,
  `seed` a decimal string, and `tanks` holds blue then orange. `training` is how the
  behavior was made (`tank::Behavior::training`; `"Scripted"` for every behavior today).
  For a placeholder-bot `WasmMatch` it is `"null"`.
- **`CatalogView`** (`tankCatalogJson()`): `budget` (points per tank, 9); per-level tables
  (index 0 = level 1) `damage` (Attack), `max_speed`, `turn_rate` and `fire_cooldown`
  (Speed), and `max_hp` (Defense); `loadouts` (the 19 valid `A-S-D` strings, by Attack
  then Speed); `presets` (`[name, loadout]` pairs); `behaviors` (`[key, name, training]`
  triples); and `default_query`. The numbers come from `tank::loadout`, so the page has one
  source of truth.
- `canonicalTankQuery(q)` is `MatchSpec::from_query(q)?.to_query()`. The viewer uses it to
  canonicalise links (`web/tank-ui.js`).

A working example: this Node script was run from the repo root against the committed
`web/pkg` on 2026-09-30 (`node tank-example.mjs`; Node also prints a harmless
`MODULE_TYPELESS_PACKAGE_JSON` warning to stderr, because `web/pkg` has no `package.json`):

```js
// From the repo root: node tank-example.mjs
import { readFileSync } from "node:fs";
import { initSync, WasmMatch, tankCatalogJson, snapLoadout, canonicalTankQuery } from "./web/pkg/engine_wasm.js";

initSync({ module: readFileSync("web/pkg/engine_wasm_bg.wasm") });

const catalog = JSON.parse(tankCatalogJson());
console.log(catalog.budget, catalog.loadouts.length, catalog.max_hp, catalog.default_query);
console.log(snapLoadout(1, 0, 0), canonicalTankQuery("?blue=Kiter-5-3-1&tab=customize"));
try { canonicalTankQuery("blue=kiter-5-3-2"); } catch (e) { console.log("throws:", e.message); }

const m = WasmMatch.tank("seed=42&blue=kiter-5-3-1&orange=charger-4-1-4");
console.log(m.setupJson());
console.log(JSON.parse(m.stateJson()).tanks.map((t) => [t.hp, t.max_hp]));
while (!m.step(500)) {}
console.log(m.tick(), m.outcomeJson(), m.stateHash());
m.free();
```

Output:

```
9 19 [ 460, 550, 650, 790, 940 ] seed=42&blue=kiter-3-3-3&orange=charger-3-3-3
5-2-2 seed=42&blue=kiter-5-3-1&orange=charger-3-3-3
throws: blue: stats sum to 10, must be exactly 9
{"query":"seed=42&blue=kiter-5-3-1&orange=charger-4-1-4","seed":"42","tanks":[{"behavior":"kiter","name":"Kiter","training":"Scripted","loadout":"5-3-1"},{"behavior":"charger","name":"Charger","training":"Scripted","loadout":"4-1-4"}]}
[ [ 460, 460 ], [ 790, 790 ] ]
1696 {"winner":0,"ticks":1696,"reason":"last_standing"} 96cfb953c76dfcc9
```

Native `tank::MatchSpec::from_query("seed=42&blue=kiter-5-3-1&orange=charger-4-1-4")`
`.run()` gives the same outcome (tick 1696) and hash, and so does headless Chrome. Tests:
`tank_duel_matches_the_tank_crate` and `catalog_and_snap` in `engine-wasm`, plus the
catalog/snap/query checks in `scripts/check-viewer.mjs`.

`StateView` JSON:

```jsonc
{
  "tick": 0, "max_ticks": 7200, "width": 800.0, "height": 600.0,
  "obstacles": [{"x": 250.0, "y": 200.0, "w": 50.0, "h": 200.0}, …], // x, y = min corner
  "tanks": [{"id": 0, "team": 0, "x": …, "y": …,
             "heading": …, "turret": …,          // radians, CCW from +X (display only)
             "hp": 100, "max_hp": 100, "alive": true}, …], // max_hp: this tank's own
  "projectiles": [{"x": …, "y": …, "vx": …, "vy": …, "team": 0}], // vx, vy per tick
  "outcome": null                                  // or {"winner", "ticks", "reason"}
}
```

Coordinates are the engine's (Y-up). `arena.js` flips Y and negates angles to draw on the
Y-down canvas. It draws with the Canvas 2D API from plain JavaScript; no `web-sys` is used
(ADR-002, corrected 2026-09-30).

### Racing: `WasmRace` (M5)

Source: `engine-wasm/src/race.rs`. It has the same two layers as tanks: `RaceViewer` (plain
Rust, unit-tested natively) and its `#[wasm_bindgen]` handle `WasmRace`. A viewer loop
written for `WasmMatch` works unchanged: `step(n)` until it returns true, then
`stateJson()` each frame. It uses only what `games/racing` already exposes; there are no
new accessors in racing.

| JS | Returns | Notes |
| --- | --- | --- |
| `WasmRace.fromBuilds(seed, buildsJson)` | `WasmRace` | A Ring race. `seed` is a decimal string; `buildsJson` is a JSON array of 1–4 racing builds (`{rules_version, levels, behavior}`), and car `i` drives `builds[i]`. Each build is checked with the same validator as `validateBuild("racing", …)`. Throws `car 1: invalid build [{"code":…,"key":…}]`, `a race takes 1 to 4 builds, got 5`, `builds must be a JSON array…`, and `car 0: a champion behavior needs its genome…` for a champion (the loader resolves those). The grid is shuffled by the seed (`slot`). Same race as `racing::balance::run` and engine-py with the same seed and builds |
| `r.step(n)` | `boolean` | Up to `n` ticks; stops at the end; true once over. Finished cars get no throttle and coast to a stop as ghosts (they hit walls, not cars). No allocation per tick, beyond the replay's action log doubling (`tests/race_alloc.rs`) |
| `r.tick()` / `r.isOver()` | `number` / `boolean` | |
| `r.stateJson()` | JSON | Per frame, below |
| `r.trackJson()` | JSON | Static geometry, below; draw it once |
| `r.outcomeJson()` | JSON | `"null"` while racing, else `{"winner", "ticks", "reason", "placings", "finish_ticks"}`. `reason` is `"finished"` (every car finished) or `"tick_limit"`. `winner` is the first finisher, or null if nobody finished |
| `r.setupJson()` | JSON | `{"seed", "cars": [{"behavior", "name", "training": "Scripted", "setup": "3-3-3"}]}` (setup = power-top_speed-grip) |
| `r.stateHash()` | 16 hex digits | Equals the replay's `final_hash` at the end |
| `r.replayJson()` | JSON | The race so far as a [format 5](replay-format.md) racing replay; the same bytes engine-py writes for the same race |
| `checkRaceReplayJson(json)` | JSON | Re-simulates a racing replay (from wasm, Rust or Python): `{"format", "game": "racing", "seed", "cars", "ticks", "outcome", "final_hash", "setup_hash", "verify_error"}`. `verify_error` is null when it verifies. Throws if it doesn't load as a racing replay. Tank replays still go to `checkReplayJson` |

Coordinates and angles follow the tank convention: Y-up, origin bottom-left, world units,
radians counter-clockwise from +X. The Ring runs counter-clockwise. `stateJson()`, with
`cars` in build order:

```json
{"tick":600,"max_ticks":3600,"over":false,
 "cars":[{"id":0,"pos":{"x":568.7607,"y":99.17433},"heading":0.74340546,"speed":153.28816,
          "lap":1,"laps_total":3,"next_gate":2,"started":true,"finished":false,
          "finish_tick":null,"placing":3,"slot":1}, …],
 "outcome":null}
```

- `speed` is in units per second.
- `lap` counts completed laps.
- `next_gate` indexes `trackJson().gates`.
- `started` is false until the car first crosses the start line.
- `placing` is the current place (1 = leading), final once the car has finished.
- At the end, `outcome` is the `outcomeJson()` object.

`trackJson()`:

```json
{"name":"Ring","width":800.0,"height":600.0,
 "centreline":[[400.0,100.0],[550.0,100.0],[700.0,250.0], …],
 "half_width":60.0,
 "inner":[[400.0,160.0],[525.1472,160.0], …],
 "outer":[[400.0,40.0],[574.8528,40.0], …],
 "gates":[{"inner":[400.0,160.0],"outer":[400.0,40.0],"start_finish":true},
          {"inner":[525.1472,160.0],"outer":[574.8528,40.0],"start_finish":false}, …],
 "start_finish_gate":0,"car_radius":12.0,"laps":3,"max_ticks":3600,
 "tick_hz":60,"seconds_per_tick":0.016666668}
```

- World bounds are `0..width` × `0..height`.
- `centreline`, `inner` and `outer` are closed polylines: the last point joins the first,
  which is not repeated.
- Vertex `i` of `inner` and `outer` is gate `i`'s endpoint (the walls are the road edges,
  offset `half_width` from the centreline).
- `gates` are in race order; gate 0 is the start/finish line.
- Race time is `tick / tick_hz` seconds.

Tests: `race.rs`'s unit tests (the same results as `racing::balance::run`, engine-py's
native fixture, shapes, bad input, replay round trip) and `scripts/race_wasm.test.mjs`
(Node, on the committed `web/pkg`). That test covers native `final_hash` parity, plus two
racing replays written from Python (`scripts/fixtures/python-racing/`, from engine-py's
`test_determinism.py` with `COASTAL_ARENA_REPLAY_DIR`) that verify in wasm. CI runs it
through `scripts/catalog_racing.test.mjs`, which imports it.

### Viewer URL parameters (`web/arena.js`, `web/tank-ui.js`)

- **Tank Arena links:** `?seed=<decimal>&blue=<tank>&orange=<tank>` (the query format
  above), canonicalised through `canonicalTankQuery`.
- **Legacy placeholder-bot links:** `?seed=<decimal>&a=<Chaser|Wanderer>&b=<Chaser|Wanderer>`,
  used only when neither `blue` nor `orange` is present and `a` or `b` is. `a` is team 0
  (blue) and `b` team 1 (orange).
- **Either kind** takes `speed=<n>` (the speed buttons offer 1, 2 and 4), `paused=1`,
  `t=<ticks>` (jump ahead that many ticks on load) and `tab=customize`.
`window.__arena.state` exposes the last parsed state for headless checks.

## Building `web/pkg`

```sh
rustup toolchain install    # 1.98.1 + wasm32-unknown-unknown, from rust-toolchain.toml (what CI runs)
cargo install wasm-bindgen-cli --version 0.2.100 --locked
./scripts/build-wasm.sh
```

`scripts/build-wasm.sh`:

1. sets `RUSTFLAGS` to remap `$CARGO_HOME` → `/cargo` and the repo root → `/src`, so paths
   embedded in panic messages don't depend on who built it;
2. `cargo build -p engine-wasm --release --target wasm32-unknown-unknown` with one codegen
   unit (`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`);
3. finds the `.wasm` cargo actually built, so `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` or
   `build.target-dir` work (since 2026-10-03; it used to package `target/…` even when
   cargo had built elsewhere, so it could ship a stale file). It reads the target
   directory from `cargo metadata --format-version 1 --no-deps`, and the built file and
   whether it was rebuilt from cargo's JSON messages (`--message-format=json-render-diagnostics`,
   parsed with `grep`/`sed`, no `jq`). It fails loudly if cargo reports no `engine_wasm`
   artifact, if the reported file isn't the expected one under that target directory, if
   the file is missing, or if cargo rebuilt it but its mtime is older than the build start
   (an up-to-date build, `"fresh": true`, is fine). `web/pkg` comes out byte-identical with
   any target directory, because the paths are remapped in step 1 and the binary carries no
   target-directory paths;
4. `wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section
   --out-dir web/pkg <that .wasm>`. The two
   `--remove-*` flags (since 2026-09-30) drop the `name` custom section (Rust function names,
   about 71 KB, used only by profilers and stack traces) and the `producers` section
   (toolchain telemetry, 112 bytes). Code and data are unchanged.

**Debugging a wasm stack trace.** Without the `name` section, stack frames read
`wasm://wasm/…:wasm-function[84]:0x22244` instead of `engine_wasm::check_replay_json`. The
error message itself is unchanged: `engine-wasm` has no panic hook, so a Rust panic surfaces
as `RuntimeError: unreachable` in either build, and `Result` errors keep their text. To get
names back locally, run the step-3 `wasm-bindgen` command without the two `--remove-*` flags
(into another `--out-dir`, not `web/pkg`). The function indices are the same in both
builds, so a stripped trace maps back to names.

Output: `web/pkg/engine_wasm.js` and `web/pkg/engine_wasm_bg.wasm`, both committed.

Version pins that must agree:

- `rust-toolchain.toml` pins `channel = "1.98.1"`, because a different rustc gives different
  wasm bytes;
- `engine-wasm/Cargo.toml` pins `wasm-bindgen = "=0.2.100"`, and the CLI must be the same
  version (the glue and the crate must match).

**Rebuild `web/pkg` in any PR that touches `engine/` or `engine-wasm/`, including
comment-only changes.** Doc comments on `#[wasm_bindgen]` items are copied into the JS glue as
JSDoc. Panic locations (file:line) are compiled into the wasm, so moving code lines changes
the bytes. PR #12 was an example: rustdoc-only edits changed both files.

Size: `engine_wasm_bg.wasm` is 307,335 bytes (120,393 with `gzip -9 -n`) since racing's
`GAMES` entry (2026-10-03): +7,177 bytes (+2,418 gzipped). That includes racing's committed
catalog JSON, its `validate_build` and params, and the validator moving out of line now
that two games share it. `game_catalog::validate`, `check_levels` and `Build::points`
take `&dyn RuleSet`, and `validation_json` hands its game-independent part to one
non-generic function. With generics, each game crate compiled its own copy of the
validator and parser: 316,666 bytes, +16,508.

Before that: 300,158 bytes (117,975) since the slim
catalog and the one-codegen-unit wasm build (2026-10-03), down from 338,099 (124,611) on
main after #56: −37,941 bytes (−11.2%), −6,636 gzipped. Measured against the same tree
without #51 (293,288 / 111,521; 276,151 / 109,750 with one codegen unit), the catalog's own
cost drops from +44,811 bytes to +25,689 (+24,007 with one codegen unit), and one codegen
unit wins back 17,137 bytes from the existing code; net, the file is 6,870 bytes (6,454
gzipped) larger than without the catalog. Two changes, measured separately:
- **Slim catalog** (318,977 / 119,787 alone): `catalogJson` and `defaultBuild` return
  committed strings instead of serializing; validation checks a `const` `Rules` (any
  `game_catalog::RuleSet`; a full `Catalog` checks the same, a test pins it); the `validateBuild` reply and error lists
  are written by hand (tested byte for byte against `serde_json`); spec and config checks lean
  on `Loadout`; no `core::fmt` or `Debug` on the catalog path. A hand-written JSON parser was
  measured and dropped: it came out 1.8 KB *larger* than the `serde_json` visitor, whose
  machinery the engine already carries.
- **`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`** in `build-wasm.sh` (wasm only; native builds
  keep 16): −18,819 bytes on top, −17,137 on the code without the catalog. Same hashes on every
  parity fixture and in the JS export harness, and no slower in Node (median of 9 runs of 120
  matches). Measured and not taken: `lto` (no further change), `opt-level = "s"`/`"z"` for
  engine-wasm (−20 KB to −27 KB more, but the wasm sim runs 6% / 16% slower), and `wasm-opt`
  (binaryen 120: −9% raw but +1% gzipped, and a new CI tool).

Before that: 332,441 bytes (122,414) with the build catalog (#51), up from 286,828 (109,245) after the tank `Flat` view: +45,613 bytes
(+15.9%), +13,169 gzipped (+12.1%). By `twiggy diff`, it is mostly `serde_json`
serialization of the catalog and validation structs, the validator and the small build
parser (`game_catalog::Loose`), plus the wasm wrappers and `fromBuilds`. A first draft with
`serde_json::Value` grew the file by 79,191 bytes; a tank-only typed version by 36,937; the
shared game-agnostic shape (stats and values as lists, so racing needs no new types) costs
the remaining 8.7 KB. Before that, 284,864 bytes (108,790) since
`build-wasm.sh` strips the `name` and `producers` sections (2026-09-30), down from 356,307
(117,316): −71,443 bytes (−20.1%), −8,526 gzipped. (`gzip -n` leaves the file name out of the
header. The older gzipped figures in this paragraph were measured with `gzip -9 -c <file>`,
which stores the name: 20 bytes for `engine_wasm_bg.wasm`, so #31's "117,336" is 117,316
without it.) History: 356,307 with `checkReplayJson` (the
parity check, #31), up from 303,694 (105,297): +52,613 bytes, 14,000 of it more
function names and the rest mostly `serde_json` deserializers for `Replay`, `Action` and
`Outcome`; 161,993 (64,526 gzipped) before `withConfig`; 247,799 (88,900)
with it, mostly `serde_json`'s deserializer; 303,811 with rules v1 (#18, the `tank` crate and
its catalog); 303,841 after the bots moved to `games/tank` (#22); 302,941 after the evolution
loop (#27); 303,694 with the generic core (ADR-014 B1, #30).

## CI check (`wasm` job in `ci.yml`)

1. `cargo build -p engine --target wasm32-unknown-unknown`;
2. install `wasm-bindgen-cli` 0.2.100 (cached);
3. run `./scripts/build-wasm.sh`;
4. fail if `git diff --exit-code -- web/pkg` shows a change, or `git status --porcelain --
   web/pkg` shows untracked files;
5. set up Node 22 and run the [parity check](determinism.md#native-vs-wasm-parity)
   (`node scripts/check-parity.mjs`, added in #32 from [CI specs](ci-specs.md) (a));
6. run the headless browser check (`scripts/check-viewer-browser.py`, Playwright with the
   image's Chrome).

So a PR with a stale or non-reproducible `web/pkg`, or one whose wasm disagrees with the
native parity manifest, goes red. The job runs on `ubuntu-24.04` by name (#29).

## Deploy: GitHub Pages

`pages.yml` runs on push to `main` (and manual dispatch). It checks out the repo and uploads
`web/` as-is with `actions/upload-pages-artifact` (`path: web`), then deploys it. **It has no
build step and never builds `web/pkg`.** Pages serves exactly the committed files. Keeping
them fresh is the `wasm` CI job's job. `pages.yml` doesn't wait for CI itself. It relies on
`main` requiring the `lint`, `test` and `wasm` checks before a merge (ADR-007).

The site is served under a subpath, `https://coastal-agentics.github.io/arena/` (later
`coastalagentics.com/arena/`). So every asset reference in `web/` must be relative:

- `arena.html`: `./style.css`, `./favicon.svg`, `./arena.js`, `./index.html`;
- `arena.js`: `import … from "./pkg/engine_wasm.js"`;
- the glue finds the wasm relative to its own URL (`import.meta.url`).

A leading `/` (e.g. `/pkg/engine_wasm.js`) would resolve to
`coastal-agentics.github.io/pkg/…` and 404. The one deliberate exception is the shared
header and footer navigation between the three sites (`/` company site, `/nyborgs/`,
`/arena/`; ADR-016): those links are root-relative so they keep working when the org site
moves to its own domain. They are plain links, never assets, so the viewer check never
fetches them; when you serve `web/` on its own they point outside it. A local check: copy `web/` to
`<tmp>/arena/`, run `python3 -m http.server` in `<tmp>`, and open
`http://localhost:8000/arena/arena.html`.
