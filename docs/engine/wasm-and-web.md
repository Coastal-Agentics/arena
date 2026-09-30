# engine-wasm and the web viewer

Source: `engine-wasm/` (`Cargo.toml`, `src/lib.rs`), `scripts/build-wasm.sh`,
`rust-toolchain.toml`, `web/arena.html`, `web/arena.js`, `web/pkg/`,
`.github/workflows/ci.yml` (`wasm` job) and `.github/workflows/pages.yml`.

The viewer landed on `main` in PR #8. It plays **live** matches between the built-in bots; it
doesn't load replay files.

## How the pieces plug together

```
engine (rlib)  ──►  engine-wasm (cdylib + rlib, wasm-bindgen =0.2.100)
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

- **`Viewer`**, plain Rust and unit-tested natively. It wraps a `Match` on
  `MatchConfig::duel()` and two boxed built-in policies, chosen by name with `BotKind::parse`
  (`"chaser"` or `"wanderer"`, case-insensitive, trimmed).
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
| `new WasmMatch(seed, team0, team1)` | `WasmMatch` | `seed` is a **decimal string**; bots `"Chaser"`/`"Wanderer"`. Throws on bad input |
| `m.step(n)` | `boolean` | Advance up to `n` ticks; `true` once the match is over |
| `m.tick()` | `number` | Ticks so far |
| `m.isOver()` | `boolean` | |
| `m.stateJson()` | `string` | JSON `StateView`, below |
| `m.outcomeJson()` | `string` | `"null"` while running, else `{"winner":…,"ticks":…,"reason":…}` |
| `m.stateHash()` | `string` | 16 hex digits; at the end of Chaser vs Wanderer, equal to `engine-cli`'s `hash` |
| `m.free()` | | Release the Rust object |
| `engineVersion()` | `string` | `engine` crate version |

`StateView` JSON:

```jsonc
{
  "tick": 0, "max_ticks": 7200, "width": 800.0, "height": 600.0,
  "obstacles": [{"x": 250.0, "y": 200.0, "w": 50.0, "h": 200.0}, …], // x, y = min corner
  "tanks": [{"id": 0, "team": 0, "x": …, "y": …,
             "heading": …, "turret": …,          // radians, CCW from +X (display only)
             "hp": 100, "max_hp": 100, "alive": true}, …],
  "projectiles": [{"x": …, "y": …, "vx": …, "vy": …, "team": 0}], // vx, vy per tick
  "outcome": null                                  // or {"winner", "ticks", "reason"}
}
```

Coordinates are the engine's (Y-up). `arena.js` flips Y and negates angles to draw on the
Y-down canvas. It draws with the Canvas 2D API from plain JavaScript; ADR-002 says "via
`web-sys`", but no `web-sys` is used.

### Viewer URL parameters (`web/arena.js`)

`?seed=<decimal>&a=<Chaser|Wanderer>&b=<Chaser|Wanderer>&speed=<n>&paused=1&t=<ticks>`
(the speed buttons offer 1, 2 and 4).
`a` is team 0 (blue), `b` team 1 (orange), and `t` jumps ahead that many ticks on load.
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
the bytes. This PR is an example: rustdoc-only edits changed both files.

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
