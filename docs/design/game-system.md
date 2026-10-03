# Game system design: one interface for tanks, racing and other training setups

**Status:** Proposed (2026-10-03). For Nye as a gate, via Soundwave. Docs only: no code changes.
**Owner:** Shockwave (Engine Lead). **Game side:** Blitzwing (Tank Designer-Developer), co-author and reviewer.
**Builds on:** ADR-009, ADR-014 (B1 done in #30: `engine::generic::Rules`), `docs/engine/`, GATE-003 (§6 bridge, M3), Saltmarsh ADR-001 and ARCHITECTURE.md (`gaming`, the planned `saltmarsh-arena` wheel).

**In one paragraph:** every game is a `Rules` impl in its own `games/<name>` crate in this repo, run by the same deterministic engine loop, replayed in the same envelope, and checked by the same native-vs-wasm parity CI. Two small, opt-in additions let training tools use any game: a defaulted per-agent `reward`, and a fixed-size `f32` view for bindings. One Python wheel built here (`saltmarsh-arena`) exposes each game as a PettingZoo/Gymnasium env, and Saltmarsh's `gaming` part wraps it. Tank refits first with no replay byte changes. Racing is game #2 and the ADR-014 trigger for B2–B5.

**Naming:** the customizable agents are **Nyborgs**: one Nyborg can play in several arenas (tank, racing), and over time both Nyborgs and maps become customizable.

## 1. Shared interface: build on `Rules`, not a new trait

Every game is multi-agent with N agents. Here is what `Rules` (on `main`) already covers and what changes:

| Concept | Where (Rust) | Today | Change |
|---|---|---|---|
| Agents | `Rules::agents`, `Rules::is_active` | N per match; an inactive agent gets the default action | none |
| Actions | `Rules::Action` (`Copy`, `Default`) + `sanitize` | Tank: 3 × f32 + bool, fixed-size | none |
| Observations | `Rules::Observation` for Rust policies | Tank's holds `Vec`s (enemies, allies, projectiles, obstacles) | Add an opt-in `Flat` view for bindings (below) |
| Rewards | `Rules::reward` | Missing. Evolution scores match results | Add a method that returns 0.0 by default (below) |
| Episode end | `Outcome` (match) + `is_active` (agent) | `EndReason`: `last_standing`, `all_destroyed`, `tick_limit` | **truncated** = `tick_limit`. **terminated** = any other reason, or the agent went inactive. Racing adds `Finished`: every active car has finished, or 600 ticks (10 s) have passed since the winner crossed the line, whichever comes first |
| Seeding, determinism | `MatchRng`, lent to `init` and `step` only | Same seed + config + per-tick actions → same final hash (`Replay::verify`; parity CI checks it native vs wasm) | none |
| Replays | One envelope, `Replay<R>`: format, config, seed, actions, `final_hash`, `setup_hash` | Format 4; no `game` field (ADR-014 known limit) | Format 5 lands **with racing**: `game` (absent = tank) and a per-game `rules_version` |

The two additions, as a sketch (not code on `main`):

```rust
pub trait Rules {
    // ... existing items unchanged ...
    /// Reward for `agent` on the tick just stepped. Computed from the state (and that
    /// step's events), so it is deterministic. Never recorded in replays.
    fn reward(_c: &Self::Config, _s: &Self::State, _e: &[Self::Event], _agent: usize) -> f32 { 0.0 }
}

/// Opt-in, fixed-size f32 view for bindings (Python, wasm). The match loop never calls it.
pub trait Flat: Rules {
    const OBS_LEN: usize;
    const ACTION_LEN: usize;
    fn encode_obs(c: &Self::Config, s: &Self::State, agent: usize, tick: u32, out: &mut [f32]);
    fn decode_action(input: &[f32]) -> Self::Action;
}
```

- **No weight in `engine/`.** These are about 15 lines with no new dependencies. Nothing in the tick loop calls them. Python, numpy and pyo3 never enter `engine/`.
- **Rewards.** Blitzwing defines tank's and racing's rewards. Evolution keeps using match-result fitness.
- **Training plugs in two ways.** Evolution uses `Match::run` in Rust, as today. RL tools use the Python env (§6).
- **The viewer plugs in through engine-wasm**, one wrapper per game. See §4.

**Gymnasium/PettingZoo mapping** (in the bindings crate, following GATE-003 §6):

| Python | Rust |
|---|---|
| `reset(seed, options)` | `Match::<R>::new(config, seed)`. Options pick the config and the scripted opponents |
| `observation_space` / `action_space` | `Box(-1, 1, (OBS_LEN,))` / `Box(-1, 1, (ACTION_LEN,))`, float32 |
| `step(actions)` | `decode_action` per agent, then `Match::step` with frame-skip in Rust. `encode_obs` writes into a reused buffer that becomes a numpy array |
| `rewards`, `terminations`, `truncations` | `R::reward` per agent; the end rules from the table above |
| `infos` | `tick`, `setup_hash` at reset, `final_hash` and outcome at the end |

N agents give a PettingZoo `ParallelEnv`. A Gymnasium env is the same match with one learning agent and the others driven by built-in Rust policies.

**"No per-tick allocation" is a target, not met today.**
- `Match::step` collects a fresh action `Vec` and pushes one `Vec` per tick into the history, and `step_policies` allocates too (`match_loop.rs`).
- `TankRules` allocates in `step` (`old`, `moves`, `spawned`, `keep`), in `observe`, and in `outcome` (`sim.rs`).
- *Progress:* E1 shipped in M1 (#45). E2 (M2) makes `TankRules::step` and `outcome` allocation-free. `observe` still allocates, as noted below.

Two hash-neutral engine steps fix the step loop, each with a before/after speed check (`engine-cli --matches 200 --seed 0`, release build, median of 5 runs):
- **E1:** a flat history buffer in `Match` (one `Vec<Action>` with stride N, reserved once) plus a reusable action scratch. The replay JSON shape is unchanged. `Match::history()` changes shape, but only engine code calls it today.
- **E2:** reusable scratch buffers in `TankRules`, with the same iteration order and the same float operations.

Tank's `Observation` still allocates after E2. Changing it to fixed arrays is an optional later step and Blitzwing's call, because it touches every tank policy.

## 2. Tank as game #1 (no behavior change)

The refit only *adds* three things:
- `tank::encode_obs`/`decode_action` (176 and 4 floats, the GATE-003 §6 layout);
- a tank `reward`: the GATE-003 shaping plus ±1 at the end, computed from state and events;
- E2 (the scratch buffers).

The rich `Observation` stays as it is. **No replay bytes change:** the format stays 4. Every pin in tank-refit.md's table stays identical, and each refit PR shows a before/after table:
- the bot pins and the seeds 0–199 digest;
- the 7 parity fixtures;
- the M1 champion `charger-2-5-2`;
- `BALANCE.md`;
- old URLs.

Parity CI proves the native and wasm side on every PR.

The full constraints and milestones T1–T3 are in [tank-refit.md](tank-refit.md) (Blitzwing).

## 3. Racing as game #2 (summary)

A new crate, `games/racing` (game id `racing`), implements `Rules` + `Flat`. 2–4 cars race 3 laps on the "Ring" track, with no weapons:
- **Track:** a closed centreline of 9 points with a 120 u width, straight-segment walls, and gates crossed in order.
- **Cars:** circle colliders with throttle, steer and grip. The physics is plain f32 maths with table headings, and the RNG is used only to shuffle the grid. The setup has 3 stats on the tanks' 9-point budget.
- **Obs and actions:** `OBS_LEN = 43` (rays, next gates, opponents) and `ACTION_LEN = 2` (throttle, steer).
- **End:** `Finished` (terminated, as in §1). Placings live in the race state.
- **Score:** evolution scores by place, with a 65% done-test vs Gen 0. The RL reward is the progress gained per tick plus a finish bonus.

**All racing numbers are untested starting values,** to be tuned in a racing `BALANCE.md`. Racing replays are format 5, and racing is the ADR-014 trigger that unblocks B2–B5. Segment geometry (walls, rays, circle-vs-segment) stays in `games/racing` and moves into `engine::arena` only if another game needs it.

The full spec and milestones R1–R4 are in [racing.md](racing.md) (Blitzwing).

## 4. Viewer and Customize tab (summary)

- **Shell.** One viewer page with a game picker. `arena.js` becomes a shell (tabs, loop, URL), and each game gets a module: `web/games/tank.js` is moved, not rewritten, and `web/games/racing.js` is new.
- **One wasm package.** `WasmMatch` stays as it is. Racing gets `WasmRace` with the same method names. Customize builds from a per-game schema, `catalogJson(game)`.
  - *Progress:* the build catalog exists for tank: `games()`, `catalogJson(game)`, `defaultBuild(game)`, `validateBuild(game, buildJson)` and `WasmMatch.fromBuilds`, generated from the `games/tank` level tables (`tank::catalog`). Every JS path that starts a match goes through its validator. See [wasm-and-web.md](../engine/wasm-and-web.md#builds-the-per-game-catalog). Racing adds its own catalog in M3.
- **URLs.** A URL with no `game` means tank, so every existing link stays byte-identical. Replays pick the game from format 5's `game`.
- **Shared Gen badge and slider.** It reads `web/data/<game>/evolution/`.
- **Data path.** Moving tank's data to `web/data/tank/` changes the nightly's output path, so the CoS schedules it. Until then the shell maps tank to today's path.

The details and milestones V1–V4 are in [viewer-multi-game.md](viewer-multi-game.md) (Blitzwing).

## 5. Other training setups (slot kept open)

Any setup that can be a `Rules` + `Flat` impl gets its own `games/<name>` crate and inherits replays, parity and bindings. Candidates, one line each:
- **Herding:** dogs steer a scripted flock into a pen. Co-op, no damage.
- **Marsh Push:** a top-down pusher shoves a T-block into a goal (the PushT task).
- **Tide Map:** explore rooms and build an occupancy map, then navigate to a goal.
- **Marsh Tag:** 2–4 agents play pursuit or capture-the-flag on map levels.
- **Obstacle course:** reach a goal through fixed obstacles; time and collisions are scored.

**Adding a new game, the checklist:**
1. A SPEC.
2. `Rules` plus the `Flat` encoding, with an index table.
3. A `reward`.
4. At least 2 scripted baselines as Gen 0.
5. A fitness, plus a 65% done-test on held-out seeds.
6. Determinism and parity tests.
7. A game id and a `rules_version`.
8. A wasm wrapper.
9. A renderer.
10. A Customize schema.
11. Balance targets and exploit tests.
12. A `BALANCE.md`.
13. A field note and a card.

## 6. Where it lives

**Recommendation: the games live in arena. Saltmarsh wraps them.** The evidence:

| Piece | Lives in | Why |
|---|---|---|
| Engine (`Rules`, `Match`, replays, hashing) | arena `engine/` | It is here (#30), with the determinism docs |
| Each game's rules | arena `games/<name>` | ADR-009; the tank already lives here |
| wasm viewer + parity CI | arena `engine-wasm/`, `web/`, `ci.yml` | Parity needs the Rust sim and the pinned replays in one repo |
| Python wheel | arena `engine-py/`, published as **`saltmarsh-arena`** | GATE-003 M3 already plans `engine-py/` here (pyo3 + maturin, abi3-py310). Saltmarsh ADR-001 expects this wheel. Building it here makes them one wheel, not two |
| Python envs for training and eval | Saltmarsh `gaming` | It registers the envs, adds py_trees agents, and links them to Saltmarsh `eval` (seeds, cost) and `data` (LeRobotDataset export) |

Splitting the Rust games across repos would duplicate the tick loop and the parity CI. Moving them into Saltmarsh would put Rust and wasm in a Python repo. Saltmarsh `[gaming]` keeps building without the wheel until it is published, which is a separate gate.

## 7. Milestones

| # | What | Owner | Accepted when | ADR-014 |
|---|---|---|---|---|
| **M1** (small) | `Rules::reward` (default 0.0), the `Flat` trait, and E1 (flat history buffer) | Shockwave | All hash pins, parity fixtures and replay bytes unchanged; speed equal or better; wasm size reported | Within B1; unlocks nothing |
| M2 | Tank refit: E2 (Shockwave), plus T1 obs and T2 reward (Blitzwing) | Shockwave, Blitzwing | tank-refit.md's before/after table identical; no replay bytes change | Within B1 |
| M3 | Racing v0: R1 (rules, format 5, `Finished`) and R2 (baselines, `Flat`, reward, BALANCE.md). Engine side: the `Finished` variant and the format 5 envelope | Blitzwing (rules), Shockwave (engine) | racing.md's acceptance; tank format 4 files still load and verify | **The trigger: unblocks B2–B5** |
| M4 | Bindings: `engine-py/` → `saltmarsh-arena`, generic over `Rules + Flat`, for tank and racing (GATE-003 M3, extended) | Shockwave | GATE-003 M3's tests pass for both games; Saltmarsh `[gaming]` can use the wheel locally | none |
| M5 | Viewer for two games: V2 (= R3), with `WasmRace` (Shockwave), and V3, with `catalogJson(game)` (Shockwave). Then B2–B5 (T3) if Nye approves | Blitzwing, Shockwave | Racing links replay exactly in the browser; racing parity fixtures in CI; tank unchanged | B2–B5 |
| Later | R4 (racing evolution, after M3); fixed-array tank obs (Blitzwing's call); the third setup from §5 | — | — | — |

**Independent of this order:**
- **V4,** the shared Gen badge and slider, is Blitzwing's next task (the GATE-003 M2 UI).
- **V1,** the tank-only viewer shell refactor with no visible change, can land any time before M5.
- **The tank data move** to `web/data/tank/` touches the nightly job, so it is for the CoS to schedule.

## Open questions for Nye
1. **Racing gate and scope:** does racing get its own gate (like GATE-002 for tanks)? For v0, is one track enough, should it mirror the tanks' 9-point budget, and is shuffling the grid by seed OK?
2. **Order:** racing (M3) before the tank Python bridge (M4), or the bridge first? GATE-003 M3 is approved and listed as next in STATE.
3. **Wheel and license:** build one wheel here, named `saltmarsh-arena` (GATE-003's working name is `engine-py`)? And should arena's crates become MIT OR Apache-2.0 before it ships? That needs every copyright holder's consent, and publishing stays a separate gate.
4. **ADR-014 after racing:** approve B2–B5 (M5), or keep them deferred? Should `Outcome` later become a per-game associated type (a B-step, not proposed now)?
5. **Cost:** Saltmarsh `eval` requires a per-step cost. Should game envs report one (e.g. racing collisions) or a declared 0.0?

## Game side, for Blitzwing to confirm
- **racing.md, end rule wording.** §1 says the race ends when every **active** car has finished, or 600 ticks (10 s) after the winner crosses the line, whichever comes first; `TickLimit` stays truncation. racing.md says "every car".
- **racing.md, a case it doesn't cover:** the 3,600-tick cap arriving after a winner has finished but before the 10 s window closes. racing.md's `TickLimit` says "nobody finished" and `winner: None`.
- **viewer-multi-game.md:** its table puts V1 "in M5". This doc lets V1 land earlier.
- **Tank and racing rewards** as written in tank-refit.md and racing.md, computed in Rust and never recorded. Evolution fitness is unchanged.
- **When to move to fixed-array tank obs,** if ever.
