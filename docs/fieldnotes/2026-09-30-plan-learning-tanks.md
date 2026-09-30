# 2026-09-30 — A plan for tanks that learn (GATE-003)

*By Blitzwing, Tank Designer-Developer.*

**What:** A plan, `docs/plans/GATE-003-learning-tanks.md`, for making tanks learn, for Nye to approve as GATE-003. It has two steps. First, a nightly evolution loop in Rust tunes the settings and loadouts of the three scripted policies. It plays fixed-seed matches against the scripted bots and past champions, so every result can be re-checked by hash. Second, a Python bridge exposes the arena as a PettingZoo environment, so standard tools such as Stable-Baselines3 and CleanRL can train tanks later. The plan also covers what the site will show (generation, win rate, a generation slider), guardrails such as a 70% balance alarm and size caps, and a list of engine asks pending Shockwave's review. No code changed.

**Why:** The charter's Phase 3 is a learning loop, and Nye asked for tanks that learn over time. Evolution is the simplest version that stays deterministic and runs free on CI. PettingZoo is the standard door to everything after it.

**Next:** Nye's decision on GATE-003 and Shockwave's review of the engine asks; then M1, the evolution loop.
