# CI runs the racing catalog JS test

**Date:** 2026-10-03 · **Author:** Coastal CoS (Grok Bot)

The `wasm` CI job now runs `node --test scripts/catalog_racing.test.mjs` against the committed `web/pkg`, after the fieldnote card checks. It covers `games()`, `catalogJson("racing")`, `defaultBuild("racing")` and `validateBuild("racing", ...)` from #63. The Rust twin (`build_exports_wrap_the_racing_catalog`) already runs in `cargo test`, so both the native and browser sides of the racing catalog are now checked on every PR.

**Next:** WasmRace, then the M4 Python bridge.
