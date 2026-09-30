# engine-wasm and the web viewer

Source: `engine-wasm/` (`Cargo.toml`, `src/lib.rs`), `scripts/build-wasm.sh`,
`rust-toolchain.toml`, `web/arena.html`, `web/arena.js`, `web/tank-ui.js`, `web/pkg/`,
`games/tank/src/matchup.rs` and `games/tank/src/loadout.rs` (the Tank Arena API),
`.github/workflows/ci.yml` (`wasm` job) and `.github/workflows/pages.yml`.

The viewer landed on `main` in PR #8. It plays **live** matches: between the placeholder bots
(`tank::Chaser`, `tank::Wanderer`), or Tank Arena duels (rules v1, #18). It doesn't load
replay files.

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
| `WasmMatch.withConfig(configJson, seed, team0, team1)` | `WasmMatch` | Same, with a custom `MatchConfig` as a JSON string ([config](replay-format.md#config)), e.g. per-tank params. Throws `config json: …` on bad JSON or a missing field |
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
| `tankCatalogJson()` | `string` | JSON `CatalogView`: the Customize tab's tables and lists (below) |
| `snapLoadout(attack, speed, defense)` | `string` | Snap barycentric weights (Attack, Speed and Defense corners of the Customize triangle) to the nearest valid loadout, `"A-S-D"` (`tank::Loadout::snap`) |
| `canonicalTankQuery(query)` | `string` | Canonical `seed=…&blue=…&orange=…` for a query; throws on invalid input |
| `engineVersion()` | `string` | `engine` crate version |

`withConfig(duelConfigJson(), seed, a, b)` plays exactly like `new WasmMatch(seed, a, b)`
(test `custom_config_with_per_tank_params`; checked in headless Chrome for six seeds). A
loadout from JS:

```js
const cfg = JSON.parse(duelConfigJson());
// A spawn's params replace the shared set as a whole: start from cfg.params.
cfg.tanks[0].params = { ...cfg.params, projectile_damage: 28, max_hp: 60 };
cfg.tanks[1].params = { ...cfg.params, projectile_damage: 24, max_speed: 90, turn_rate: 273, max_hp: 120 };
const m = WasmMatch.withConfig(JSON.stringify(cfg), "42", "Chaser", "Wanderer");
```

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
2. `cargo build -p engine-wasm --release --target wasm32-unknown-unknown`;
3. `wasm-bindgen --target web --no-typescript --out-dir web/pkg
   target/wasm32-unknown-unknown/release/engine_wasm.wasm`.

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

Size: `engine_wasm_bg.wasm` is 303,694 bytes (105,297 with `gzip -9`) with the generic core
(ADR-014 B1), against 302,941 (105,295) on `main` just before it (#28): 753 bytes more,
2 bytes more gzipped. History: 161,993 (64,526 gzipped) before `withConfig`; 247,799 (88,900)
with it, mostly `serde_json`'s deserializer; 303,811 with rules v1 (#18, the `tank` crate and
its catalog); 303,841 after the bots moved to `games/tank` (#22); 302,941 after the evolution
loop (#27).

## CI check (`wasm` job in `ci.yml`)

1. `cargo build -p engine --target wasm32-unknown-unknown`;
2. install `wasm-bindgen-cli` 0.2.100 (cached);
3. run `./scripts/build-wasm.sh`;
4. fail if `git diff --exit-code -- web/pkg` shows a change, or `git status --porcelain --
   web/pkg` shows untracked files.

So a PR with a stale or non-reproducible `web/pkg` goes red.

## Deploy: GitHub Pages

`pages.yml` runs on push to `main` (and manual dispatch). It checks out the repo and uploads
`web/` as-is with `actions/upload-pages-artifact` (`path: web`), then deploys it. **It has no
build step and never builds `web/pkg`.** Pages serves exactly the committed files. Keeping
them fresh is the `wasm` CI job's job. `pages.yml` doesn't wait for CI itself. It relies on
`main` requiring the `lint`, `test` and `wasm` checks before a merge (ADR-007).

The site is served under a subpath, `https://starscream-agentics.github.io/arena/`. So every
reference in `web/` must be relative:

- `arena.html`: `./style.css`, `./favicon.svg`, `./arena.js`, `./index.html`;
- `arena.js`: `import … from "./pkg/engine_wasm.js"`;
- the glue finds the wasm relative to its own URL (`import.meta.url`).

A leading `/` (e.g. `/pkg/engine_wasm.js`) would resolve to
`starscream-agentics.github.io/pkg/…` and 404. A local check: copy `web/` to
`<tmp>/arena/`, run `python3 -m http.server` in `<tmp>`, and open
`http://localhost:8000/arena/arena.html`.
