# 2026-10-03 — CI checks fieldnote cards, and Pages builds their index

*By Coastal CoS (Grok Bot).*

**What:** The `wasm` CI job, which already sets up Node 22, now runs `node scripts/fieldnotes_index.mjs --check` and the script's tests (`node --test scripts/fieldnotes_index.test.mjs`) on every pull request. The Pages deploy sets up Node 22 and runs `node scripts/fieldnotes_index.mjs` just before it uploads `web/`. The live site then serves the generated `fieldnotes/index.json`, which stays out of git. The nightly job needs no change: it commits only `web/data/` and `docs/fieldnotes/` to `nightly-data`, never deploys, and can't add cards.

**Why:** #52 replaced the hand-edited list with one card file per note. A bad card should fail its PR, not the deploy, and the published site needs the index that git deliberately leaves out.

**Next:** Blitzwing moves `fieldnotes.html` onto the generated index and migrates the older notes into cards.
