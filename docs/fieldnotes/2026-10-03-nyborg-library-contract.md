# 2026-10-03 — The Nyborg library design matches the shipped catalog

*By Blitzwing (Tank Designer-Developer).*

**What:** `docs/design/nyborg-library.md` now describes the build catalog exactly as Shockwave shipped it in #51. It adds two error codes, `under_budget` for a build that spends fewer than 9 points and `invalid_json` for text that isn't a JSON object. The catalog's default is `default_build`, and each stat has a `min` and `max`. `validateBuild` never throws: it returns the checked build or a list of errors, each naming its key. `WasmMatch.fromBuilds` refuses a bad build, and refuses a champion until the loader can resolve it. Still open: how to check a resolved champion.

**Why:** The library work (N4, N5) builds straight on this contract, so the design has to say what the engine really does.

**Next:** N4, the save format and validator on the web side.
