# Engine docs

The arena runs on a small deterministic 2D simulation written in Rust (`engine/`). It steps
at a fixed 60 Hz, takes all its randomness from one seeded RNG, and records every match as a
replay that re-simulates to the same state. The same crate runs headless in `engine-cli`
(CI's source of truth) and in the browser through `engine-wasm` and `web/arena.html`.

These pages describe the code on `main`. Where a page says what a function does, it comes from
reading that function; the rustdoc (`cargo doc --no-deps -p engine --open`) has the same facts
next to the code.

**Current shape, stated plainly:** the engine has a generic core, but its only rules are the
tank's. `engine::generic` (ADR-014 step B1) holds the game-agnostic parts: the `Rules` trait (with a defaulted `reward`), the opt-in
`Flat` view for bindings,
`Match<R>` (tick counter, seeded RNG, event log, outcome, flat action history), `Replay<R>`,
`ReplayPlayer<R>`, `Policy<R>`, `MatchRng` and `StateHasher`. `TankRules` is the one `Rules`
implementation, and `engine::Match`, `engine::Replay` and `engine::ReplayPlayer` are aliases
for the tank instances, so existing code didn't change. Tanks, projectiles, the tank step
rules, the tank `Observation`/`Action` pair and `TankParams` still live in `engine/`.
`games/tank` (crate `tank`) builds on them: the Tank Arena rules v1 (loadouts, the pillar arena
and spawns, the charger, kiter and sniper policies; #18) and, since ADR-014 Phase A, the two
placeholder bots `Chaser` and `Wanderer`. The Tank Arena spec (`games/tank/SPEC.md`, GATE-002)
is merged, and the engine side of its asks is in: line of sight, per-tank params, `max_hp` and
`los` in observations, optional stationary accuracy (replay format 4). ADR-009 plans for tank
specifics to live in `games/tank`; its 2026-09-30 correction records that they are in `engine/`
today and that the move is future work.
[ADR-014](../DECISIONS.md) (Accepted) plans how. Step B1, the generic core, is the only
approved step and is what's described here. Moving the tank rules to `games/tank` is
deferred until a second Rust game exists.

## Architecture

```mermaid
flowchart LR
  subgraph rust["Rust workspace"]
    engine["engine<br/>generic core: Rules, Match&lt;R&gt;, Replay&lt;R&gt;, Policy&lt;R&gt;<br/>TankRules: arena, tanks, projectiles<br/>(Match = Match&lt;TankRules&gt;)"]
    tank["games/tank (crate tank)<br/>rules v1, loadouts, policies,<br/>bots (Chaser, Wanderer)"]
    cli["engine-cli<br/>headless runner (binary)"]
    wasm["engine-wasm<br/>wasm-bindgen bindings:<br/>WasmMatch, Viewer"]
  end
  tank -->|depends on| engine
  cli -->|depends on| engine
  cli -->|depends on| tank
  wasm -->|depends on| engine
  wasm -->|depends on| tank

  cli -->|"summary JSON (stdout)"| out["results: seed, winner,<br/>ticks, reason, hash"]
  cli -->|"--replay-dir"| replays["match-SEED.json<br/>(replay format 4)"]

  wasm -->|"scripts/build-wasm.sh<br/>cargo build wasm32 + wasm-bindgen 0.2.100"| pkg["web/pkg<br/>engine_wasm.js + engine_wasm_bg.wasm<br/>(committed)"]
  pkg -->|"import ./pkg/engine_wasm.js"| js["web/arena.js<br/>canvas renderer"]
  js --> html["web/arena.html"]
  html -->|"pages.yml uploads web/ as-is"| pages["GitHub Pages<br/>coastal-agentics.github.io/arena/"]
```

Data flow in one line each:

- **Headless:** `engine-cli` builds `MatchConfig::duel()` matches, drives them with
  `tank::Chaser` vs `tank::Wanderer`, prints a JSON summary, and optionally writes one replay
  per match.
- **Browser:** `arena.js` creates a `WasmMatch` (seed string + two bot names, or
  `WasmMatch.tank(query)` for a Tank Arena duel), calls `step(n)` from its animation loop, and
  draws the JSON from `stateJson()` on a canvas.
- `engine-cli` and `engine-wasm` depend on `games/tank`; `engine` does not, and can't (that
  would be a crate cycle, ADR-014).

## Pages

| Page | What it covers |
| --- | --- |
| [World](world.md) | Arena, coordinates, headings (64-BAU aim resolution), line of sight, tanks, projectiles, `TankParams`, per-tank params, stationary accuracy, `MatchConfig::duel`, events, end conditions |
| [Tick loop](tick-loop.md) | The fixed 60 Hz step, the generic `Match::step` and the order of work inside `TankRules::step`, how callers drive it |
| [Seeds and determinism](determinism.md) | The ChaCha8 RNG (`MatchRng`) and what draws from it, simultaneous movement, trig table, the one `sqrt`, state hash, the native-vs-wasm parity check, JS-safe string seeds |
| [Observations, actions and policies](policies.md) | `Observation` (including `max_hp` and `los`), `Action`, the `Policy` trait (generic over `Rules`), and the placeholder `Chaser` and `Wanderer` (in `games/tank`) |
| [Replay format](replay-format.md) | Tank Arena's format 4 field by field, the format 5 game envelope (`game`, `rules_version`), the setup hash, versioning (formats 2 and 3 still read), `verify`, `ReplayPlayer` |
| [engine-cli](engine-cli.md) | Flags, output, replay files, examples |
| [CI specs](ci-specs.md) | Workflow changes for the workflow owner to apply: the parity step for the `wasm` job, and the `engine-py` wheel workflow |
| [Python bindings](python.md) | `engine-py` and the `coastal-arena` wheel: the PettingZoo, Gymnasium and zero-copy `FlatEnv` APIs for tank and racing, the buffers, determinism tests and throughput |
| [engine-wasm and the web viewer](wasm-and-web.md) | The JS API (including `withConfig`, `duelConfigJson` and the rules-v1 Tank Arena calls with a run example), building `web/pkg`, relative paths, the CI check, Pages deploy |

Related: [ADR-001, -003, -008, -009, -014](../DECISIONS.md) in `docs/DECISIONS.md`.
