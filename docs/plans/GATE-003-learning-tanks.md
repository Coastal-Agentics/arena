# GATE-003 — Learning tanks (plan)

*By Blitzwing, Tank Designer-Developer. Status: **proposed**, for Nye's approval as GATE-003. Plan only: no code in this PR. Written 2026-09-30 against `main` at `af3fd93`.*

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
- **Held-out check:** the champion is scored again on **1,000 seeds it never trained on**. The charter's done-test is **Gen N wins ≥ 65% of 1,000 fixed-seed matches against Gen 0**. This plan defines Gen 0 as the hand-written defaults, which makes that bar meaningful (open question Q1).
- **Re-verification:** each published result lists genome, loadout, seeds, commit and final state hashes. A CI check replays them and fails on any mismatch. ADR-003 promises the same match on the **same platform**, so verification runs on the same runner type (ubuntu-latest, x64).

## 3. How it runs

- **Where:** the nightly workflow already exists (`.github/workflows/nightly.yml`, 07:00 UTC = 3:00 AM ET, `timeout-minutes: 60`). Its self-play step is an `echo` placeholder today. It merges `main` into the unprotected `nightly-data` branch and may commit only `web/data/` and `docs/fieldnotes/` (ADR-007).
- **Budget:** public-repo `ubuntu-latest` runners have 4 CPUs and 16 GB, are free, and allow up to 6 hours per job ([runner specs](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [limits](https://docs.github.com/en/actions/reference/limits)). On the box, one core plays about **600 Tank Arena matches per second** (1,200 triangle matches in 2.0 s). A generation (32 candidates × about 280 matches ≈ 9,000 matches) therefore takes roughly 4–15 s on 4 cores. The plan caps training at **30 minutes and 100 generations a night**, well inside the job's 60 minutes.
- **Promotion:** the nightly publishes to `nightly-data` only. The CoS brings `nightly-data` into `main` with a normal PR, as ADR-007 says. A champion becomes the viewer's **default** "Gen N" only through a reviewed PR (Blitzwing + CoS), never automatically.
- **Change to `.github/`:** M1 needs the placeholder step replaced. That file is outside Blitzwing's paths (`games/tank/`, `web/`), so the change is proposed as a separate PR for whoever owns workflows.

## 4. What Nye sees

- **Training badge:** each tank card already shows a **Scripted** badge. Its tooltip promises "generation, win rate vs the other policies and a fitness sparkline". The badge becomes, for example, **Gen 42 · 68% vs scripted**, with a small fitness chart.
- **Generation slider:** this does not exist yet; only the charter and SPEC describe it. It will walk from Gen 0 to today's champion, and the Watch tab will replay the chosen generation live.
- **"Scripted vs Gen N" matchup:** a preset on the Watch tab, shareable by URL like today's matches.
- **No big files:** the viewer re-simulates from (genome, loadout, seed) in the browser instead of downloading replays. A measured replay costs about 120 bytes per tick for a duel, so about 240 KB for a typical 2,000-tick match and 870 KB for a full 7,200-tick one.

## 5. Guardrails

| Rule | How it is enforced |
|---|---|
| No single champion above **70%** averaged against the scripted field. (This is the SPEC's loadout rule, applied to evolved tanks.) | A champion above 70% is still published but flagged as a **balance alarm**. It does not become the viewer default until a rules review PR decides. |
| Evolved tanks only tune existing policies. | Parameters are clamped to published bounds, and the code path is the scripted policy code. |
| Every claim can be replayed. | The CI check re-runs the published seeds and compares hashes. |
| `web/data/` stays small. | ≤ 200 KB per nightly commit and ≤ 5 MB in total. Every generation keeps its summary line, but genomes are kept only for the Hall of Fame and every 10th generation. There are no replay files. |
| Nightly cannot run away. | A 30-minute training cap. The job fails loudly and never force-pushes. Disabling the workflow is the kill switch. |
| Provenance. | Each champion ships a `docs/CARD.md` card with inputs, seeds, commit, compute and fitness formula. |

## 6. The PettingZoo bridge

[PettingZoo](https://pettingzoo.farama.org/) (Farama Foundation, MIT; latest release 1.27.0, Python 3.10–3.14) is the multi-agent sister of Gymnasium. Its **Parallel API** fits the arena exactly: every tank acts at once on each tick. The signatures below come from the [Parallel API docs](https://pettingzoo.farama.org/api/parallel/): `reset(seed=None, options=None) -> (observations, infos)`, `step(actions) -> (observations, rewards, terminations, truncations, infos)` (each a dict keyed by agent), `observation_space(agent)`, `action_space(agent)`, `render()`, `close()` and `state()`.

**Plan:** a new crate `engine-py/`, next to `engine-wasm/`, built with **pyo3 + maturin**. It wraps `engine::Match`, and the Python env is a thin `ParallelEnv` over it. `engine/` itself gets no Python dependency, so it still builds for wasm and keeps its hashes. The simulation stays in Rust. Python only sends actions and reads observations. Every Python-driven match is an ordinary `Replay` (seed, config, per-tick actions, final hash), so it can be re-verified in Rust or watched in the browser without Python.

| PettingZoo | Tank Arena mapping |
|---|---|
| `possible_agents` / `agents` | One name per tank, `"<team>_<id>"` from the match config, e.g. duel `["blue_0", "red_1"]`. `agents` drops a tank once it is destroyed. Tanks can be controlled by built-in scripted bots through `options`, and are then not agents. |
| `observation_space(agent)` | `Box(-1, 1, (176,), float32)`, flattened in Rust in a documented, tested layout. **Self** (11): pos, vel, heading sin/cos, turret sin/cos, hp/max_hp, max_hp, cooldown. **4 enemy + 4 ally slots** (12 each, 96): present flag, rel, vel, heading sin/cos, turret sin/cos, hp/max_hp, max_hp, los. **8 projectile slots** (6 each, 48): present, rel, vel, is-enemy. **Walls** (4), **tick/7200** (1), **4 obstacle slots** (16). Missing slots are zeros with present = 0. Lists stay nearest-first, as in `Observation`. Values are scaled by arena size and stat maxima. |
| `action_space(agent)` | `Box(-1, 1, (4,), float32)`: throttle, turn, turret_turn and fire, where **fire = value > 0**. Why a thresholded Box: SB3's SAC and TD3 accept only `Box` actions, and no SB3 algorithm accepts `Tuple`/`Dict` actions ([SB3 algorithms](https://stable-baselines3.readthedocs.io/en/master/guide/algos.html)). One Box works with PPO, SAC, TD3 and CleanRL's continuous PPO. A `MultiDiscrete` variant (e.g. 3×3×3×2) can come later for DQN-style tools. |
| `reset(seed=, options=)` | Calls `Match::new(cfg, seed)` with `cfg` from `tank::rules` (fixed mirrored spawns). `options`: `loadouts` (e.g. `{"blue_0": "5-3-1"}`), `opponents` (e.g. `{"red_1": "kiter"}`), `frame_skip`. `seed=None` draws a seed from the env's seeded generator and reports it in `infos`. |
| `step(actions)` | Applies each action for **4 ticks** (frame-skip; 15 decisions per second; at most 1,800 steps). It stops early if the match ends. **terminations** is true when the match is won, lost or wiped out. **truncations** is true at the 7,200-tick cap (a draw under the SPEC). |
| rewards (proposal) | Terminal: **+1 win, −1 loss, 0 draw**. Shaping per step: **+0.5 × damage dealt / enemy max HP − 0.5 × damage taken / own max HP**, so shaping adds at most ±0.5 per match and stays zero-sum. The formula goes on the provenance card. |
| `infos` | `tick`, `hp`, `setup_hash` (at reset), `winner`/`reason` and **`final_hash`** (the replay's state hash) at the end, plus the seed. |
| `render()` | `render_mode="ansi"` gives a one-line text summary. `rgb_array` is deferred. The real "human" view is exporting the replay and watching it in the web viewer (that needs replay loading in `web/`, a Blitzwing task). |
| `state()` | Optional; the full-state vector for centralized critics. Not planned in M3. |

**How Saltmarsh tools plug in.** Stable-Baselines3 is single-agent, so it trains on PettingZoo envs through SuperSuit (`pettingzoo_env_to_vec_env_v1`, then `concat_vec_envs_v1(..., base_class="stable_baselines3")`). That gives one shared policy for all tanks, which also means self-play ([PettingZoo SB3 tutorial](https://pettingzoo.farama.org/tutorials/sb3/index.html)). CleanRL has PettingZoo examples ([CleanRL tutorial](https://pettingzoo.farama.org/tutorials/cleanrl/index.html)). The repo docs describe **Saltmarsh** only as "a simulated walker or arm trained with open tools" in MuJoCo and Python, not started and behind its own gate (CHARTER, ADR-010, STATE). They do **not** mention Gymnasium, CleanRL or Stable-Baselines3. This plan takes those tool names from Soundwave's brief and assumes nothing else about Saltmarsh's setup. The benefit is shared tooling: the tank bridge would be the company's first standard RL environment, and Saltmarsh can reuse the same training scripts and card format.

**Evolution vs the bridge.** Evolution needs nothing new and ships a learning loop Nye can watch within M1–M2. The bridge opens the arena to every standard RL tool, but it brings Python packaging, new dependencies (pyo3, maturin, numpy) and a crate that needs an owner. Neither blocks the other. Evolution stays M1 because it is simpler.

## 7. Milestones

| | What | Accepted when | Owner |
|---|---|---|---|
| **M1** | Evolution loop in `games/tank` (`evolve` example) and results format in `web/data/evolution/`. | Two runs with the same seed give byte-identical output. The champion wins **≥ 65% of 1,000 held-out seeds vs Gen 0**. The re-verify check passes. 100 generations fit in 30 min on 4 cores. Tests pass. | Blitzwing. The `nightly.yml` step goes to the workflow owner (CoS). |
| **M2** | Nightly runs it; the viewer shows the badge, slider, "Scripted vs Gen N" preset and win-rate chart. | 7 nightly runs in a row are green. The CI re-verify job checks the latest champion. The chart is public. The first champion card is merged. | Blitzwing (`web/`), CoS (workflow, `nightly-data` fold-in). |
| **M3** | `engine-py/` (pyo3 + maturin) and a `TankArenaParallelEnv`. | `parallel_api_test` and `parallel_seed_test` pass ([PettingZoo tests](https://pettingzoo.farama.org/content/environment_tests/)). A 1,000-step Python episode's `final_hash` equals Rust's `Replay::verify` of the same actions. Native and wasm hashes are unchanged. An SB3 PPO smoke run (10k steps, CPU) completes. | **Unassigned.** Proposed: Shockwave for the crate, Blitzwing for the observation/action/reward spec and env tests. Soundwave to decide. |
| Later | Real RL training runs (Python, possibly GPU); a learned-network tank in the viewer. | Its own gate: compute budget, weight-file policy, provenance. | — |

M3 should start after ADR-014 **B1** lands (in progress), so the bindings wrap the generic core rather than a moving target. M1 uses only the public `Match` API and does not wait.

## 8. Engine asks (for Shockwave) — *pending Shockwave's review*

1. **Stable API through B1:** keep `Match::new/observe/step/state_hash/replay`, `Replay::verify`, `TankSpawn.params` and the `Observation`/`Action` shapes stable, or ship a migration note. Evolution and the bindings build on them.
2. **Native↔wasm parity as a standing check:** a pinned set of replays must produce the same hash natively and in wasm. A champion verified in CI must play identically in the browser. #22 checked this once; this ask makes it permanent.
3. **Optional, only if profiling shows a need:** a way to run a match without recording per-tick actions, for fitness runs of about 10⁵ matches a night.
4. **New crate `engine-py/`** beside `engine-wasm/`: pyo3 + maturin bindings over `Match` (`new`, `step` with frame-skip and early stop, observe, `state_hash`, replay JSON). pyo3 stays out of `engine/`. Ownership needs a decision: Shockwave's brief covers `engine/` and `engine-cli/` only, and no brief names `engine-wasm/` either.
5. **Fixed-size observation encoding** (`Observation → [f32; 176]`) in Rust with a layout test, so Python and any future Rust network policy agree. Blitzwing writes the layout into `games/tank/SPEC.md`.
6. **New dependencies** pyo3, maturin and `numpy` (Rust crate): beyond the brief's preferred list, so they need approval.
7. **CI for the bridge:** build the wheel and run the PettingZoo API and seed tests (a workflow change for the `.github/` owner).

## 9. Risks

- **Overfitting** to training seeds or opponents. Mitigation: held-out seeds and the Hall of Fame.
- **Stalling for draws** (Kiter mirrors draw 72.8% today). Mitigation: draws score 0.25, and the draw rate is published.
- **Evolution finds a balance hole.** Kiter already beats Charger 76.2%, near the 80% ceiling. The 70% alarm makes this a rules decision, not a surprise on the site.
- **Determinism is same-platform only** (ADR-003). Mitigation: verify on one runner type, plus the parity check (ask 2).
- **B1 churn** could break M3's bindings. Mitigation: M3 waits for B1.
- **Nightly schedule:** GitHub disables scheduled workflows in a public repo after **60 days without repository activity** ([docs](https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-workflow-runs/disabling-and-enabling-a-workflow)). Runner specs can also change.
- **Bridge upkeep:** Python versions, pyo3 upgrades and wheels. RL results are not reproducible across CPU and GPU (SB3 says so), even though the sim is. Replays still record exactly what happened.
- **Scope creep** into Saltmarsh. The bridge is for tanks; Saltmarsh keeps its own gate.

## 10. Deferred

- RL training runs, neural-network tanks in the viewer, and model-weight hosting (after M3, under a new gate).
- A `MultiDiscrete` action variant, `rgb_array` rendering and `state()`.
- Evolving new behaviors (anything beyond the three policies' parameters), and 2v2 team fitness.
- Any link to Saltmarsh beyond shared tooling.

## Open questions for Nye

- **Q1:** Is Gen 0 the hand-written defaults (the stricter bar, proposed) or a random start (which shows the charter's "chaos" phase on the slider)?
- **Q2:** Is the 70% balance alarm acceptable as "publish, but don't make it the default until reviewed"?
- **Q3:** Who owns `engine-py/`?
