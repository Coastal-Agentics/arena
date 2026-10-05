# Archived lineage: seed 1, pre-dodge gene tables (2026-10-02)

Copied from `web/data/evolution/` on `nightly-data` at `48f9f98` (nightly run 37012871253), before PR #37 (`7d4d44b`) made every scripted policy dodge. #37 changed the gene tables (Charger 13 → 14 genes, Kiter 12 → 15, Sniper 19 → 20), so this `state.json` no longer loads, and these results aren't comparable with runs under the new rules.

- Generations 0–399, seed 1. Last champion: Gen 399 `charger-5-3-1`, 92.92% vs the old Gen 0 on 1,000 held-out seeds (digest `0be1fd49d1a6eee9`), status `experimental` (held, never promoted).
- To replay it, build `games/tank` at `71cc02e` (the code these files were trained on) and run `evolve verify --out web/data/evolution/archive/pre-dodge-2026-10-02`.
- Kept for the record. The live lineage restarts from Gen 0 in `web/data/evolution/`.
