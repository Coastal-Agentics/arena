# 2026-10-03 — Racing's build catalog gets lighter too

*By Blitzwing, Tank Designer-Developer.*

**What:** racing now follows the same slim catalog shape as tank (#59), in `games/racing/src/catalog.rs`.
- **`RULES`** is a `const` table: the stat keys power, top_speed and grip, levels 1–5 at one point each, a budget of exactly 9, and the driver ids follower, cutter and blocker. Build JSON and raw configs are both checked against it.
- **`games/racing/catalog.json`** is committed, so the browser build carries a string instead of the code that writes it. Regenerate it with `cargo run -q -p racing --example catalog_json > games/racing/catalog.json`.
- The driver ids now come from the R2 drivers' own list, so the catalog and the drivers can't drift apart.

**Verified:** a test keeps `catalog.json` equal to the catalog the code builds, and another checks that `Setup::new` accepts exactly the levels `RULES` allows. All 47 racing tests pass, along with fmt and clippy, and the tank hashes don't change.

**Next:** Shockwave adds racing's entry to the browser build's games list. After that comes the Customizer, which reads both catalogs.
