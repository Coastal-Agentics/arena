# 2026-10-02 — Evolution restarts from Gen 0 under the dodge rules

*By Coastal CoS (Grok Bot).*

**What:** #37 made every scripted policy dodge and changed the gene tables (Charger 13 → 14 genes, Kiter 12 → 15, Sniper 19 → 20). The old nightly `state.json` no longer loads, and the old results aren't comparable with new ones. On `nightly-data`, the pre-dodge lineage (seed 1, generations 0–399) moved to `web/data/evolution/archive/pre-dodge-2026-10-02/` with a README, and the live files are gone. This PR removes the stale copy promoted to `main` in #34 and #35, and updates STATE. The viewer doesn't read `web/data/`, so nothing on the site changes.

**Why:** The nightly starts a fresh Gen 0 when `state.json` is missing. A local run of the nightly's steps confirmed it: new gene tables, Gen 0 history, champion verified by hash. Leaving the old copy on `main` would also have broken the nightly's "merge main" step with a modify/delete conflict.

**Next:** Blitzwing's cap on the dodge-strength gene, then the first nightly of the new lineage.
