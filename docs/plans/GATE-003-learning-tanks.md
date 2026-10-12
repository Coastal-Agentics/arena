# GATE-003 — Learning tanks (plan)

*By Blitzwing, Tank Designer-Developer. Status: **approved (GATE-003, 2026-09-30)**. Written 2026-09-30 against `main` at `af3fd93`, merged in #26 as `47e9ee2`; revised the same day with Shockwave's review of §8 and the answers to the open questions (see Resolved).*

**Summary.** Today every tank in the arena runs one of three hand-written policies (Charger, Kiter, Sniper). The plan makes tanks learn in two steps. **First (M1–M2):** a small, plain **evolution** loop in Rust. It takes the numbers that tune the existing policies, plus the 9-point loadout, and each night it breeds better settings by playing thousands of fixed-seed matches against the scripted bots and against past champions. Every result can be replayed exactly from its seed, so any claim ("Gen 40 beats the scripted bots 68% of the time") can be re-checked by hash. The site shows the generation, the win rate and a slider through the generations. **Second (M3):** a **Python bridge** that exposes the same Rust arena as a standard PettingZoo multi-agent environment. Off-the-shelf learning tools (CleanRL, Stable-Baselines3) can then train tanks with neural networks, while the Rust core stays deterministic. The RL training runs themselves come later, under their own gate.

## 1. What learns

| | M1–M2: evolution | M3 bridge → later RL runs |
|---|---|---|
| What changes | The **settings** of an existing policy: Charger has 13 numbers, Kiter 12, Sniper 19 (range, aim tolerance, weave and so on, including 3 shared stall-breaker settings), plus the loadout (one of the 19 Attack/Speed/Defense splits of 9 points). | A **neural network** that maps what the tank sees to stick and trigger. |
| How | Genetic algorithm: keep the best, mix pairs of parents, nudge numbers at random within fixed bounds. Every step uses a seeded RNG, so a run is repeatable. | PPO or SAC from Stable-Baselines3 or CleanRL, through PettingZoo. |
| Where it runs | Rust, CPU only, in the nightly GitHub Actions job. | Python, on a laptop or GPU box. |
| What a result can do | Only what the three scripted policies can already express, with better numbers. Safe and explainable. | New behavior. More interesting, harder to explain. |

**Why evolution first.** It needs no new languages, dependencies or credentials. It reuses the policies that already pass the balance targets. It fits the charter's Phase 3 as written ("nightly self-play evolution over policy parameters, CPU-only, in CI"). Its results are byte-for-byte repeatable. RL adds Python packaging, neural-network weight files, GPU nondeterminism and a much larger search. It is worth doing, but after the arena is exposed through a standard interface (M3).

## 2. Fitness (how "better" is scored)

- **Opponents:** the three scripted policies at 3/3/3, plus a **Hall of Fame** of the last 8 champions, so a lineage cannot forget how to beat old tricks.
- **Matches:** every candidate plays each opponent on a fixed list of seeds, on **both sides** of the mirrored spawns.
- **Score:** win = 1, draw = 0.25, loss = 0. Draws score low so that stalling to the 120-second cap does not pay: Kiter mirrors already draw 72.8% of the time. Ties are broken by damage dealt. The formula is published next to the results, as the charter requires.
- **Held-out check:** the champion is scored again on **1,000 seeds it never trained on**. The charter's done-test is **Gen N wins ≥ 65% of 1,000 fixed-seed matches against Gen 0**. Gen 0 is today's hand-written defaults, which makes that bar meaningful (resolved, Q1).
- **Re-verification:** each published result lists genome, loadout, seeds, commit and final state hashes. A CI check replays them and fails on any mismatch. ADR-003 promises the same match on the **same platform**, so verification runs on the same runner type (`ubuntu-24.04`, x64; pinned in PR #29).

## 3. How it runs

- **Where:** the nightly workflow already exists (`.github/workflows/nightly.yml`, cron `37 6 * * *` = 06:37 UTC, 2:37 AM ET in summer and 1:37 AM in winter, since PR #36; `timeout-minutes: 60`). Its self-play step is an `echo` placeholder today. It merges `main` into the unprotected `nightly-data` branch and may commit only `web/data/` and `docs/fieldnotes/` (ADR-007).
- **Budget:** public-repo `ubuntu-24.04` runners have 4 CPUs and 16 GB, are free, and allow up to 6 hours per job ([runner specs](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [limits](https://docs.github.com/en/actions/reference/limits)). On the box, one core plays about **600 Tank Arena matches per second** (1,200 triangle matches in 2.0 s). A generation (32 candidates × about 280 matches ≈ 9,000 matches) therefore takes roughly 4–15 s on 4 cores. The plan caps training at **30 minutes and 100 generations a night**, well inside the job's 60 minutes.
- **Promotion:** the nightly publishes to `nightly-data` only. The CoS brings `nightly-data` into `main` with a normal PR (a `nightly-data` → `main` PR, never copied files), as ADR-007 says. A new lineage reuses `web/data/evolution/`; old lineages are archived under `web/data/evolution/archive/<name>/` (`GATE-003-nightly-spec.md`, "Starting a new lineage"). A champion becomes the viewer's **default** "Gen N" only through a reviewed PR (Blitzwing + CoS), never automatically.
- **Change to `.github/`:** M1 needs the placeholder step replaced. That file is outside Blitzwing's paths (`games/tank/`, `web/`), so it goes to the workflow owner (the CoS) as a spec: [`GATE-003-nightly-spec.md`](GATE-003-nightly-spec.md), with the exact commands, layout, caps and hold rule.

## 4. What Nye sees

- **Training badge:** each tank card already shows a **Scripted** badge. Its tooltip promises "generation, win rate vs the other policies and a fitness sparkline". The badge becomes, for example, **Gen 42 · 68% vs scripted**, with a small fitness chart.
- **Generation slider:** this does not exist yet; only the charter and SPEC describe it. It will walk from Gen 0 to today's champion, and the Watch tab will replay the chosen generation live.
- **"Scripted vs Gen N" matchup:** a preset on the Watch tab, shareable by URL like today's matches.
- **No big files:** the viewer re-simulates from (genome, loadout, seed) in the browser instead of downloading replays. A measured replay costs about 120 bytes per tick for a duel, so about 240 KB for a typical 2,000-tick match and 870 KB for a full 7,200-tick one.

## 5. Guardrails

| Rule | How it is enforced |
|---|---|
| No single champion above **70%** averaged against the scripted field. (This is the SPEC's loadout rule, applied to evolved tanks.) | A champion above 70% is **held from promotion and labeled experimental**, and the CoS tells Nye (resolved, Q2). It is still published on `nightly-data`, but it does not become the viewer default. |
| Evolved tanks only tune existing policies. | Parameters are clamped to published bounds, and the code path is the scripted policy code. |
| Every claim can be replayed. | The CI check re-runs the published seeds and compares hashes. |
| `web/data/` stays small. | ≤ 200 KB per nightly commit and ≤ 5 MB in total. Every generation keeps its summary line, but genomes are kept only for the Hall of Fame and every 10th generation. There are no replay files. |
| Nightly cannot run away. | A 30-minute training cap. The job fails loudly and never force-pushes. Disabling the workflow is the kill switch. |
| Provenance. | Each champion ships a `docs/CARD.md` card with inputs, seeds, commit, compute and fitness formula. |

## 6. The PettingZoo bridge

[PettingZoo](https://pettingzoo.farama.org/) (Farama Foundation, MIT; latest release 1.27.0, Python 3.10–3.14) is the multi-agent sister of Gymnasium. Its **Parallel API** fits the arena exactly: every tank acts at once on each tick. The signatures below come from the [Parallel API docs](https://pettingzoo.farama.org/api/parallel/): `reset(seed=None, options=None) -> (observations, infos)`, `step(actions) -> (observations, rewards, terminations, truncations, infos)` (each a dict keyed by agent), `observation_space(agent)`, `action_space(agent)`, `render()`, `close()` and `state()`.

**Plan:** a new crate `engine-py/`, next to `engine-wasm/`, built with **pyo3 + maturin** and owned by Shockwave. It wraps `engine::Match`, and the Python env is a thin `ParallelEnv` over it. Frame-skip and early stop run in Rust, not Python. The crate uses pyo3's stable ABI (`abi3-py310`), so one wheel covers Python 3.10–3.14. Observations cross as a flat `Vec<f32>` (bytes) and become numpy arrays with `np.frombuffer`, so there is no Rust `numpy` crate at first. The observation encoding is tank-specific, so it lives in `games/tank` as `tank::encode_obs` (Blitzwing, per ADR-009/ADR-014); `engine-py` and the wasm build call it from there. `engine/` itself gets no Python dependency, so it still builds for wasm and keeps its hashes. The simulation stays in Rust. Python only sends actions and reads observations. Every Python-driven match is an ordinary `Replay` (seed, config, per-tick actions, final hash), so it can be re-verified in Rust or watched in the browser without Python.

| PettingZoo | Tank Arena mapping |
|---|---|
| `possible_agents` / `agents` | One name per tank, `"<team>_<id>"` from the match config, e.g. duel `["blue_0", "red_1"]`. `agents` drops a tank once it is destroyed. Tanks can be controlled by built-in scripted bots through `options`, and are then not agents. |
| `observation_space(agent)` | `Box(-1, 1, (176,), float32)` from `tank::encode_obs`: self, 4 enemy and 4 ally slots, 8 projectile slots, walls, tick and 4 obstacle slots. The layout, order, overflow and scaling rules are in the next table. |
| `action_space(agent)` | `Box(-1, 1, (4,), float32)`: throttle, turn, turret_turn and fire, where **fire = value > 0**. Why a thresholded Box: SB3's SAC and TD3 accept only `Box` actions, and no SB3 algorithm accepts `Tuple`/`Dict` actions ([SB3 algorithms](https://stable-baselines3.readthedocs.io/en/master/guide/algos.html)). One Box works with PPO, SAC, TD3 and CleanRL's continuous PPO. A `MultiDiscrete` variant (e.g. 3×3×3×2) can come later for DQN-style tools. |
| `reset(seed=, options=)` | Calls `Match::new(cfg, seed)` with `cfg` from `tank::rules` (fixed mirrored spawns). `options`: `loadouts` (e.g. `{"blue_0": "5-3-1"}`), `opponents` (e.g. `{"red_1": "kiter"}`), `frame_skip`. `seed=None` draws a seed from the env's seeded generator and reports it in `infos`. |
| `step(actions)` | Rust applies each action for **4 ticks** (frame-skip; 15 decisions per second; at most 1,800 steps). It stops early if the match ends. **terminations** is true when the match is won, lost or wiped out. **truncations** is true at the 7,200-tick cap (a draw under the SPEC). |
| rewards (proposal) | Terminal: **+1 win, −1 loss, 0 draw**. Shaping per step: **+0.5 × damage dealt / enemy max HP − 0.5 × damage taken / own max HP**, so shaping adds at most ±0.5 per match and stays zero-sum. The formula goes on the provenance card. |
| `infos` | `tick`, `hp`, `setup_hash` (at reset), `winner`/`reason` and **`final_hash`** (the replay's state hash) at the end, plus the seed. |
| `render()` | `render_mode="ansi"` gives a one-line text summary. `rgb_array` is deferred. The real "human" view is exporting the replay and watching it in the web viewer (that needs replay loading in `web/`, a Blitzwing task). |
| `state()` | Optional; the full-state vector for centralized critics. Not planned in M3. |

**Observation layout (`tank::encode_obs`, M3).** Shockwave confirmed the total: 11 + 96 + 48 + 4 + 1 + 16 = **176**.

| Block | Floats | Features, in order | Scaling |
|---|---|---|---|
| Self | 11 | pos x, y · vel x, y · heading cos, sin · turret cos, sin · hp/max_hp · max_hp · cooldown | pos: 2·p/arena − 1 (arena 800 × 600) · vel ÷ 2.5 (units per tick at Speed 5: 150 u/s ÷ 60) · max_hp ÷ 940 (Defense 5) · cooldown ÷ 64 (the longest reload, Speed 1) |
| Enemies, allies | 4 + 4 slots × 12 = 96 | present · rel x, y · vel x, y · heading cos, sin · turret cos, sin · hp/max_hp · max_hp · los | rel ÷ arena size per axis (800, 600) · vel ÷ 2.5 · max_hp ÷ 940 · flags 0 or 1 |
| Projectiles | 8 slots × 6 = 48 | present · rel x, y · vel x, y · is-enemy | rel ÷ (800, 600) · vel ÷ 6 (360 u/s ÷ 60) |
| Walls | 4 | left, right, bottom, top distances | ÷ 800, 800, 600, 600 |
| Tick | 1 | tick | ÷ 7200 |
| Obstacles | 4 slots × 4 = 16 | min x, min y, max x, max y | like pos |

- **Order and ties:** slots are nearest first. For tanks, equal distance goes to the **lower tank id** (the engine's own sort: `dist_sq`, then `id`). Shells have no id, so equal distance keeps the engine's order: older shells first, and shells fired on the same tick by lower tank id.
- **Overflow:** the engine already keeps only the nearest 4 enemies, 4 allies and 8 projectiles; `encode_obs` fills the slots from those lists. Obstacles are ordered by distance from the tank's centre to the nearest point of the rectangle (ties by index in `Arena::obstacles`), and only the nearest 4 are kept. The Tank Arena has 2 pillars, so two slots are empty today.
- **Empty slots** are all zeros (present = 0). An empty obstacle slot is all zeros too; no real obstacle is a zero-size box at the centre.
- **Clamping:** after scaling, **every feature is clamped to [−1, 1]**, so custom params or future stats cannot break the Box.
- **No libm trig:** cos and sin come from the engine's heading table (`engine::angle::dir`), so the encoding is identical natively and in wasm.
- **Versioning:** the constants come from the level tables (`MAX_SPEED[4]`, `MAX_HP[4]`, `FIRE_COOLDOWN[0]`). If a table changes, the constants and an encoding version number change together.

**How Saltmarsh tools plug in.** Stable-Baselines3 is single-agent, so it trains on PettingZoo envs through SuperSuit (`pettingzoo_env_to_vec_env_v1`, then `concat_vec_envs_v1(..., base_class="stable_baselines3")`). That gives one shared policy for all tanks, which also means self-play ([PettingZoo SB3 tutorial](https://pettingzoo.farama.org/tutorials/sb3/index.html)). CleanRL has PettingZoo examples ([CleanRL tutorial](https://pettingzoo.farama.org/tutorials/cleanrl/index.html)). The repo docs describe **Saltmarsh** only as "a simulated walker or arm trained with open tools" in MuJoCo and Python, not started and behind its own gate (CHARTER, ADR-010, STATE). They do **not** mention Gymnasium, CleanRL or Stable-Baselines3. This plan takes those tool names from Soundwave's brief and assumes nothing else about Saltmarsh's setup. The benefit is shared tooling: the tank bridge would be the company's first standard RL environment, and Saltmarsh can reuse the same training scripts and card format.

**Evolution vs the bridge.** Evolution needs nothing new and ships a learning loop Nye can watch within M1–M2. The bridge opens the arena to every standard RL tool, but it brings Python packaging, new dependencies (pyo3, maturin, numpy) and a crate that needs an owner. Neither blocks the other. Evolution stays M1 because it is simpler.

## 7. Milestones

| | What | Accepted when | Owner |
|---|---|---|---|
| **M1** | Evolution loop in `games/tank` (`evolve` example) and results format in `web/data/evolution/`. | Two runs with the same seed give byte-identical output. The champion wins **≥ 65% of 1,000 held-out seeds vs Gen 0**. The re-verify check passes. 100 generations fit in 30 min on 4 cores. Tests pass. | Blitzwing. The `nightly.yml` step goes to the workflow owner (CoS). |
| **M2** | Nightly runs it; the viewer shows the badge, slider, "Scripted vs Gen N" preset and win-rate chart. | 7 nightly runs in a row are green. The CI re-verify job checks the latest champion. The chart is public. The first champion card is merged. | Blitzwing (`web/`), CoS (workflow, `nightly-data` fold-in). |
| **M3** | `engine-py/` (pyo3 + maturin, abi3-py310) and a `TankArenaParallelEnv`; `tank::encode_obs` in `games/tank`. | `parallel_api_test` and `parallel_seed_test` pass ([PettingZoo tests](https://pettingzoo.farama.org/content/environment_tests/)). A 1,000-step Python episode's `final_hash` equals Rust's `Replay::verify` of the same actions. `encode_obs` tests: length 176, every value in [−1, 1], the tie-break and overflow cases, and the same output natively and in wasm. One abi3 wheel imports on Python 3.10 and 3.14. `cargo test --workspace` and the existing CI jobs run unchanged. Native and wasm hashes are unchanged, and the Node parity script passes. An SB3 PPO smoke run (10k steps, CPU) completes. | **Shockwave:** `engine-py/`, the parity fixtures and script. **Blitzwing:** `tank::encode_obs`, the observation/action/reward spec and the env tests. **CoS:** the wheel CI job and the parity hook (below). |
| Later | Real RL training runs (Python, possibly GPU); a learned-network tank in the viewer. | Its own gate: compute budget, weight-file policy, provenance. | — |

M3 should start after ADR-014 **B1** lands (in progress), so the bindings wrap the generic core rather than a moving target. M1 uses only the public `Match` API and does not wait.

## 8. Engine asks (for Shockwave) — *reviewed by Shockwave 2026-09-30*

1. **Stable API through B1: agreed.** B1 keeps every current name compiling through a `Match` alias and a default type parameter on `Policy`. That covers `Match::new/observe/step/state_hash/replay`, `Replay::verify`, `TankSpawn.params` and the `Observation`/`Action` shapes. `REPLAY_FORMAT` stays at 4, and the B1 PR adds a migration note if anything shifts.
2. **Native↔wasm parity: agreed.** Shockwave adds a pinned replay fixture set and a Node script that verifies it against `web/pkg` and compares with native. Hooking it into the `wasm` CI job is a CoS item (below).
3. **No-record runner: deferred.** At about 600 matches per second, 10⁵ matches take about 3 minutes on one core, and recording per-tick actions is cheap. It is added only if profiling says so.
4. **`engine-py/`: Shockwave owns it** (resolved, Q3), and his brief widens to cover `engine-wasm/` and `engine-py/`.
   - Frame-skip and early stop live in Rust, not Python.
   - pyo3's `extension-module` feature breaks `cargo test --workspace`. So either `engine-py` is left out of the workspace's default members, or the feature is turned on only in the maturin build. Either way the existing CI is untouched.
   - It uses `abi3` (py310), so one wheel covers Python 3.10–3.14.
5. **Observation encoding: moved out of the engine asks.** It is tank-specific, so per ADR-009/ADR-014 it lives in `games/tank` as `tank::encode_obs` and is Blitzwing's M3 work (layout in §6). `engine-py` and the wasm build call it from there.
6. **Dependencies:** pyo3 is fine, confined to `engine-py`. maturin is a build tool, not a Cargo dependency. The Rust `numpy` crate is dropped at first: `engine-py` returns `Vec<f32>` or bytes, and Python converts with `np.frombuffer`. numpy is added only if profiling needs it.
7. **CI (CoS, the workflow owner):** a separate job or workflow builds the wheel and runs the PettingZoo API and seed tests, so engine CI is not slowed. The CoS also hooks Shockwave's parity script (ask 2) into the `wasm` job.

## 9. Risks

- **Overfitting** to training seeds or opponents. Mitigation: held-out seeds and the Hall of Fame.
- **Stalling for draws** (Kiter mirrors draw 72.8% today). Mitigation: draws score 0.25, and the draw rate is published.
- **Evolution finds a balance hole.** Kiter already beats Charger 76.2%, near the 80% ceiling. The 70% alarm makes this a rules decision, not a surprise on the site.
- **Determinism is same-platform only** (ADR-003). Mitigation: verify on one runner type, plus the parity check (ask 2).
- **B1 churn** could break M3's bindings. Mitigation: M3 waits for B1.
- **Nightly schedule:** GitHub disables scheduled workflows in a public repo after **60 days without repository activity** ([docs](https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-workflow-runs/disabling-and-enabling-a-workflow)). Runner specs can also change.
- **pyo3 and the workspace:** pyo3's `extension-module` feature breaks `cargo test --workspace`. Mitigation: `engine-py` stays out of the default workspace members, or the feature is enabled only by maturin, so the existing jobs never see it.
- **Bridge upkeep:** Python versions, pyo3 upgrades and wheels. RL results are not reproducible across CPU and GPU (SB3 says so), even though the sim is. Replays still record exactly what happened.
- **Scope creep** into Saltmarsh. The bridge is for tanks; Saltmarsh keeps its own gate.

## 10. Deferred

- RL training runs, neural-network tanks in the viewer, and model-weight hosting (after M3, under a new gate).
- A `MultiDiscrete` action variant, `rgb_array` rendering and `state()`.
- Evolving new behaviors (anything beyond the three policies' parameters), and 2v2 team fitness.
- Any link to Saltmarsh beyond shared tooling.

## Resolved (2026-09-30)

- **Q1: Gen 0 is today's scripted defaults** (Charger, Kiter and Sniper as shipped, at 3/3/3).
- **Q2: a champion above 70% vs the scripted field is held from promotion and labeled experimental, and the CoS tells Nye.**
- **Q3: Shockwave owns `engine-py/`.**

## Amendments

- **2026-10-07: V3 Charger approach bounds and M1 re-pin (approved by Nye).** The Charger's `stop_dist` gene floor rises from 20 to 60 and its `steer_tol` ceiling falls from 0.6 to 0.2, the shipped values. As with the 2026-10-02 dodge cap, evolution may make a tank more careful than its scripted self, never more reckless. The evidence is in `docs/design/tank-balance-2026-10.md` (PR #80). The old M1 champion (`charger-2-5-2`, digest `d15709d4b3bd6953`) has `stop_dist` 20 and `steer_tol` 0.6, so it no longer loads. The §7 M1 pin is re-made from the same seed-1, 100-generation run under the new bounds: Gen 99 `sniper-5-2-2`, 5,738 of 6,000 (95.6%) vs Gen 0, digest `0a2f3a7e2e498278`, status `experimental`. The nightly lineage is archived (`web/data/evolution/archive/pre-v3-2026-10-07/`) and restarts at Gen 0. Scripted tanks, Gen 0, the Chaser/Wanderer pins and the bot smoke digest are unchanged.
