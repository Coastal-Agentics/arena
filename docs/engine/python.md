# Python bindings: engine-py and the saltmarsh-arena wheel

Source: `engine-py/` (`src/lib.rs`, `src/games.rs`, `src/session.rs`, `src/python.rs`,
`python/saltmarsh_arena/`, `pyproject.toml`). Plan: [game-system.md](../design/game-system.md)
§1, §6 and M4, following GATE-003 §6.

`engine-py` exposes Tank Arena and Racing to Python training tools. The simulation stays in
Rust. Python sends actions and reads observations. Every Python-driven match is an ordinary
arena replay: the same seed, builds and actions give the same replay bytes and `final_hash`
as a native run. The wheel is built locally. Publishing it is a separate gate.

## Build and test

```sh
cd engine-py
pip install "maturin>=1.9.4,<2"
maturin build --release --out dist            # one abi3 wheel, CPython 3.10 to 3.14
pip install "$(ls dist/*.whl)[all]" -r requirements-test.txt -r requirements-extras.txt
SALTMARSH_ARENA_EXTRAS=all python -m pytest -q # python/tests
cargo test --release                           # the Rust core, no Python needed
```

**Thin by default.** The base install needs only numpy. `FlatEnv`, the catalog calls and
`verify_replay` never import anything else. The envs are extras:

| Install | Adds | For |
|---|---|---|
| `saltmarsh-arena` | numpy | `FlatEnv`, `games`, `catalog`, `default_build`, `validate_build`, `verify_replay` |
| `saltmarsh-arena[gym]` | gymnasium | `gym_env` |
| `saltmarsh-arena[pettingzoo]` | pettingzoo, gymnasium (PettingZoo's spaces) | `parallel_env` |
| `saltmarsh-arena[all]` | both | everything |

The env modules (`_gymnasium`, `_pettingzoo`) are imported on first use. Without their
extra, `gym_env`, `parallel_env`, `ArenaGymEnv` and `ArenaParallelEnv` raise
`saltmarsh_arena.MissingExtraError`, an `ImportError` that names the extra to install.
`python/tests/test_bare.py` checks this, and it checks that the base API loads neither
Gymnasium nor PettingZoo. `SALTMARSH_ARENA_EXTRAS=none` makes a run assert that the extras
are absent (the bare-wheel CI step). `=all` makes a missing extra fail the env tests
instead of skipping them.

**Kept apart from the engine.** `engine-py` is its own Cargo workspace. The root workspace
lists it in `exclude`, and it has its own `Cargo.lock` and `target/`. So `cargo build`,
`test`, `clippy` and `doc --workspace` never see pyo3, and the root `Cargo.lock` has no pyo3
entries. `tests/lock.rs` checks that every crate both lockfiles share has the same version,
so the engine and games in the wheel build exactly as in engine-cli and engine-wasm. pyo3
sits behind the `python` feature, which only maturin turns on. maturin also sets
`PYO3_BUILD_EXTENSION_MODULE` itself, so pyo3's deprecated `extension-module` feature isn't
used. `engine/`, the games, engine-wasm and engine-cli have no change.

## API

```python
import saltmarsh_arena as sa

sa.games()            # [{"game": "tank", "obs_len": 176, "action_len": 4, "min_agents": 2, ...}, {"game": "racing", ...}]
sa.catalog("racing"); sa.default_build("tank"); sa.validate_build("tank", build)   # the viewer's catalog calls

env = sa.parallel_env("tank", builds=[blue, orange], learning=None, frame_skip=4)  # PettingZoo ParallelEnv ([pettingzoo])
env = sa.gym_env("racing", builds=[car] * 4, agent=0)                              # Gymnasium Env, one learner ([gym])
f = sa.FlatEnv("racing", builds, learning=[0, 2])                                  # zero-copy numpy views

env.replay_json()                     # the match as replay JSON
sa.verify_replay("tank", json)        # Replay::verify natively, returns final_hash
```

- **Builds** are the catalog's build JSON (a string or a dict), checked by
  `<game>::catalog::validate_build`. Agent `i` plays `builds[i]`. Tank takes 2 builds, a
  duel built like `MatchSpec` (Blue then Orange). Racing takes 1 to 4 builds on the Ring.
  The default is the game's default build for every agent (2 tanks or 4 cars).
- **Learning agents** (`learning`, default all) are the ones Python drives. Every other agent
  plays its build's scripted behavior in Rust, seeded as natively (tank: the match seed XOR
  the side's salt; racing: `Behavior::driver`). A champion build has no genome here, so only
  a learning agent can play one.
- **Names:** tank `blue_0`, `orange_1`; racing `car_0` to `car_3`.
- **Spaces:** `Box(-1, 1, (OBS_LEN,), float32)` and `Box(-1, 1, (ACTION_LEN,), float32)`.
  These are the games' `Flat` views: tank 176 and 4 (fire when `action[3] > 0`), racing 43
  and 2.
- **Step:** `frame_skip` ticks (default 4) with the same actions, run in Rust, stopping early
  at the end. Each tick is `Match::step_policies`: an inactive agent gets the default action
  and its driver isn't asked. The reward is `Rules::reward`, summed over the step's ticks.
- **Done:** an agent terminates when the match ends for any reason but the tick limit, or
  when it leaves play (a tank destroyed, a car finished). It is truncated at the tick limit.
  It then drops out of `agents`. The Gymnasium env ends when its agent is done.
- **Infos:** `seed`, `tick` and `setup_hash` at reset. `tick` on every step. At the end:
  `final_hash`, `ticks`, `reason`, `winner` (an agent name or `None`) and `winner_team`.
- `reset(seed, options)`: `options` is accepted and ignored. Builds, learning agents and
  `frame_skip` are fixed when the env is made.

The type stubs are `python/saltmarsh_arena/_core.pyi` (checked with `mypy.stubtest`). The
package is typed (`py.typed`) and passes `mypy --strict`.

## Hot path

`FlatEnv` and the envs share one `_core.Arena`. It owns four `bytearray` buffers, made once:
observations, actions, rewards and active flags. Python views them with `np.frombuffer`, so
nothing is copied. `Arena.step()` takes no arguments. It copies the action bytes into a
reused `f32` buffer, decodes one action per row with `Flat::decode_action`, steps the match,
and writes the observations (`Flat::encode_obs`), rewards and flags back. Byte copies make
alignment irrelevant. A numpy view holds a buffer export, so Python can't resize a buffer
under it, and the length is checked on every call anyway. This uses no Rust numpy crate,
and the `bytearray` calls are in the 3.10 stable ABI.

The step allocates nothing itself. `tests/alloc.rs` counts heap allocations over whole
episodes with every agent learning. A 7,200-tick tank episode makes 16 and a 3,600-tick
race 15. All of them are the match history doubling. A scripted tank's rich `Observation`
is still built per tick, as natively. The PettingZoo and Gymnasium envs copy each
observation so it stays valid after the next step. `FlatEnv` is the zero-copy path.

## Determinism

- `tests/session.rs`: a session's replay is byte-identical to the native loop
  (`reference_episode`: `Match::step_policies` with closures, no session). It is checked for
  every learning set, frame skips 1, 4 and 7, and several seeds, in both games. An
  all-scripted session equals `MatchSpec::run` (tank) and the `racing::balance` race.
- `tests/fixtures/determinism.json` pins nine episodes' native final hashes (`tests/fixture.rs`).
  These include 1,000-step tank and racing episodes. `python/tests/test_determinism.py`
  plays the same episodes from Python through `FlatEnv`, the PettingZoo env and the
  Gymnasium env. The replay JSON must equal the native run's byte for byte, and the
  `final_hash` must equal the pin and `verify_replay`.
- `examples/verify_replay.rs` re-verifies the replays the Python tests write
  (`SALTMARSH_ARENA_REPLAY_DIR`) in a binary with no Python in it. Tank replays from Python
  also verify in wasm (`checkReplayJson`), and racing replays too (`checkRaceReplayJson`,
  `scripts/race_wasm.test.mjs`).
- `python/tests/test_envs.py` runs PettingZoo's `parallel_api_test` and `parallel_seed_test`
  and Gymnasium's `check_env` for both games.

## Throughput

Run `cargo run --release --example bench` (native) and `python bench/bench.py` (the same
loop through `FlatEnv` and the PettingZoo env). The numbers for a change go in its PR.
The PettingZoo column needs `[pettingzoo]`. `bench/sb3_smoke.py` is an optional
Stable-Baselines3 PPO smoke run. It needs `[gym]`, SB3 and torch, which the wheel doesn't
depend on.

## Not here yet

- Publishing: no PyPI config or tokens. That is a separate gate.
- Tank 2v2 and FFA lineups. Builds give a duel today, like `fromBuilds`.
- Vectorized envs. Use SuperSuit or Gymnasium's vector wrappers.
- Releasing the GIL during `step`.
- A per-step cost for Saltmarsh `eval` (game-system.md open question 5).
