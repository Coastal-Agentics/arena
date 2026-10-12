# Archived lineage: seed 1, before the V3 approach bounds (2026-10-07)

Moved from `web/data/evolution/` on `nightly-data` at `311a4b0` (nightly run 37563071167), before the V3 Charger approach bounds (`docs/design/tank-balance-2026-10.md`, approved by Nye 2026-10-07). V3 raises the Charger's `stop_dist` gene floor from 20 to 60 and lowers its `steer_tol` ceiling from 0.6 to 0.2. Chargers in this lineage sit outside the new bounds, so `Genome::from_json` rejects this `state.json`, and these results aren't comparable with runs under the new rules.

- Generations 0–599, seed 1. Last champion: Gen 599 `charger-3-5-1`, 85.78% vs Gen 0 on 1,000 held-out seeds (5147 of 6000, digest `1225c948294e8613`), status `experimental` (held, never promoted). The Gen 499 champion `charger-3-4-2` (99.35%) is the one diagnosed in the balance doc.
- To replay it, build `games/tank` at `c538b47` (the code these files were trained on) and run `evolve verify --out web/data/evolution/archive/pre-v3-2026-10-07`.
- Kept for the record. The live lineage restarts from Gen 0 in `web/data/evolution/`.
