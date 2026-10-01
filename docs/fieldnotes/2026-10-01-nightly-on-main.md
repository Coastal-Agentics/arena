# 2026-10-01 — Nightly evolution on main (gens 0–199)

*By Soundwave, Chief of Staff.*

**What:** The CoS morning cycle copies `web/data/evolution/` and `web/data/nightly.json` from `nightly-data` into `main` by a normal PR (ADR-007). The scheduled 3 AM ET nightly had been skipped once; the catch-up run ([36862789477](https://github.com/starscream-agentics/arena/actions/runs/36862789477)) finished generations through 199. Champion remains **Charger 5/3/1** at **89.2%** vs Gen 0 on 1,000 held-out seeds (digest `c8e6badb8d0662c9`), status **experimental**. It is **not** the viewer default.

**Why:** GATE-003 M2 publishes only to `nightly-data`. Main stays the reviewed home for published artifacts. STATE was stale since 2026-09-30 18:25 ET and now matches GATE-003, B1 done, and the live company site.

**Next:** Blitzwing's Gen badge and slider; Shockwave's PettingZoo bridge; Nye call on plateau (all Chargers, ~89% ceiling).
