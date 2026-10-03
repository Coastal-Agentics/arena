# Game system design: one interface for tanks, racing and other training setups

**Status:** Proposed (2026-10-03). For Nye as a gate, via Soundwave. Docs only: no code changes.
**Owner:** Shockwave (Engine Lead). **Game side:** Blitzwing (Tank Designer-Developer), co-author and reviewer.
**Builds on:** ADR-009, ADR-014 (B1 done in #30: `engine::generic::Rules`), `docs/engine/`, GATE-003 (§6 bridge, M3), Saltmarsh ADR-001 and ARCHITECTURE.md (`gaming`, the planned `saltmarsh-arena` wheel).

**In one paragraph:** every game is a `Rules` impl in its own `games/<name>` crate in this repo, run by the same deterministic engine loop, replayed in the same envelope, and checked by the same native-vs-wasm parity CI. Two small, opt-in additions let training tools use any game: a defaulted per-agent `reward`, and a fixed-size `f32` view for bindings. One Python wheel built here (`saltmarsh-arena`) exposes each game as a PettingZoo/Gymnasium env, and Saltmarsh's `gaming` part wraps it. Tank refits first with no replay byte changes. Racing is game #2 and the ADR-014 trigger for B2–B5.

## 1. Shared interface: build on `Rules`, not a new trait

Every game is multi-agent with N agents. Here is what `Rules` (on `main`) already covers and what changes:

| Concept | Where (Rust) | Today | Change |
|---|---|---|---|
| Agents | `Rules::agents`, `Rules::is_active` | N per match; an inactive agent gets the default action | none |
| Actions | `Rules::Action` (`Copy`, `Default`) + `sanitize` | Tank: 3 × f32 + bool, fixed-size | none |
| Observations | `Rules::Observation` for Rust policies | Tank's holds `Vec`s (enemies, allies, projectiles, obstacles) | Add an opt-in `Flat` view for bindings (below) |
| Rewards | `Rules::reward` | Missing. Evolution scores match results | Add a method that returns 0.0 by default (below) |
| Episode end | `Outcome` (match) + `is_active` (agent) | `EndReason`: `last_standing`, `all_destroyed`, `tick_limit` | **truncated** = `tick_limit`; **terminated** = any other reason, or the agent went inactive. Racing adds one `EndReason` variant |
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

Two hash-neutral engine steps fix the step loop, each with a before/after speed check (`engine-cli --matches 200 --seed 0`, release build, median of 5 runs):
- **E1:** a flat history buffer in `Match` (one `Vec<Action>` with stride N, reserved once) plus a reusable action scratch. The replay JSON shape is unchanged. `Match::history()` changes shape, but only engine code calls it today.
- **E2:** reusable scratch buffers in `TankRules`, with the same iteration order and the same float operations.

Tank's `Observation` still allocates after E2. Changing it to fixed arrays is an optional later step and Blitzwing's call, because it touches every tank policy.

## 2. Tank as game #1 (no behavior change)

Tank keeps its rules, policies and numbers. The refit only adds to it:
- E2 (scratch buffers);
- `Flat` through `tank::encode_obs` (the 176-float layout already planned in GATE-003 §6);
- a tank `reward`.

**The refit changes no replay bytes.** The format stays 4, and the parity fixtures and their hashes stay byte-identical. The bot and M1 champion hash pins don't move. The parity CI (native vs wasm on the pinned replays) proves it on every PR.

The full constraints are in [tank-refit.md](tank-refit.md) (Blitzwing; pending).

## 3. Racing as game #2 (summary)

A new crate, `games/racing`, implements `Rules` + `Flat`. 2–4 cars race 3 laps on the "Ring" track, with no weapons:
- **Track:** a closed centreline of 9 points with a 120 u width, straight-segment walls, and one gate per point that must be crossed in order.
- **Cars:** circle colliders with throttle, steer and grip. The physics is plain f32 maths with table headings, and the RNG is used only to shuffle the grid.
- **Obs and actions:** `OBS_LEN = 43` (rays, next gates, opponents) and `ACTION_LEN = 2` (throttle, steer).
- **End and score:** the new `Finished` end reason (terminated), or `tick_limit` (truncated). Placings live in the race state. Evolution scores by place. The RL reward is the progress gained per tick plus a finish bonus.

Racing replays are format 5. As the second Rust game, racing is the ADR-014 trigger that unblocks B2–B5. **Engine answer to racing.md's geometry question:** segment walls, rays and circle-vs-segment go in `games/racing` first. They move into `engine::arena` only if a second game needs them, which keeps the engine small.

The full spec is in [racing.md](racing.md) (Blitzwing).

## 4. Viewer and Customize tab (summary)

`engine-wasm`'s `WasmMatch` and the viewer are tank-shaped today. Each game gets a thin wasm wrapper and a renderer behind one viewer shell that picks the game from the replay's `game` field (format 5) or the URL. The Customize tab becomes per game. Parity CI gains racing fixtures.

The details are in [viewer-multi-game.md](viewer-multi-game.md) (Blitzwing; pending).

## 5. Other training setups (slot kept open)

Any setup that can be a `Rules` + `Flat` impl gets its own `games/<name>` crate and inherits replays, parity and bindings. Candidates, one line each:
- **Marsh Push:** a top-down pusher shoves a T-block into a goal (the PushT task).
- **Tide Map:** explore rooms and build an occupancy map, then navigate to a goal.
- **Marsh Tag:** 2–4 agents play pursuit or capture-the-flag on map levels.
- **Obstacle course:** reach a goal through fixed obstacles; time and collisions are scored.
- **Cooperative carry:** two agents move one object to a goal together.

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
| M2 | Tank refit: E2 (scratch buffers), `tank::encode_obs`, tank reward | Shockwave (E2), Blitzwing (obs, reward) | No replay bytes change; parity green; speed equal or better | Within B1 |
| M3 | Racing v0: `games/racing`, headless, with tests, replay format 5, new `EndReason` variant | Blitzwing (rules), Shockwave (engine bits) | Racing replays verify native and wasm; tank format 4 files still load and verify | **The trigger: unblocks B2–B5** |
| M4 | Bindings: `engine-py/` → `saltmarsh-arena`, generic over `Rules + Flat`, for tank and racing (GATE-003 M3, extended) | Shockwave | GATE-003 M3's tests pass for both games; Saltmarsh `[gaming]` can use the wheel locally | none |
| M5 | Viewer multi-game, and B2–B5 if Nye approves (tank rules move into `games/tank`) | Blitzwing, Shockwave | Hash pins unchanged; parity green | B2–B5 |
| Later | Fixed-array tank obs (Blitzwing's call); the third setup from §5 | — | — | — |

## Open questions for Nye
1. **Order:** racing (M3) before the tank Python bridge (M4), or the bridge first? GATE-003 M3 is approved and listed as next in STATE.
2. **Wheel:** build one wheel in arena, named `saltmarsh-arena`, replacing GATE-003's working name `engine-py`? Publishing it to PyPI stays a separate gate.
3. **License:** arena is MIT and Saltmarsh is Apache-2.0. Should the arena crates and wheel become MIT OR Apache-2.0 before the wheel ships? That needs the consent of everyone who holds copyright in them.
4. **B2–B5:** once racing exists, move the tank rules into `games/tank` (M5), or keep them deferred?
5. **Outcome:** should `Outcome` later become a per-game associated type (an ADR-014 B-step)? Not proposed now.
6. **Cost:** Saltmarsh `eval` requires a per-step cost. Should game envs report one (e.g. racing collisions) or a declared 0.0?

## Game side, for Blitzwing to confirm
- The tank and racing reward formulas, computed in Rust from state and events. Evolution fitness is unchanged.
- That `tank::encode_obs` (176 floats, GATE-003 §6) is tank's `Flat` impl, and that `decode_action` matches the 4-float Box (fire = value > 0).
- That E2 keeps tank's iteration and tie order, so the bot and M1 champion pins hold.
- Racing (`racing.md`, landed): that the geometry stays in `games/racing` for now, and that each car being its own team fits `Outcome.winner: Option<u8>`.
- That the viewer and Customize plan in `viewer-multi-game.md` fits `web/` as it is.
- Whether and when to move to fixed-array tank obs.
