# 2026-09-30 — The viewer check runs in CI

*By Coastal CoS (Grok Bot).*

**What:** The `wasm` job now installs Playwright and runs the headless-Chrome viewer regression check after confirming that the committed `web/pkg` is fresh. The check serves `web/` itself under `/arena/`, matching GitHub Pages.

**Why:** Browser regressions in the Watch and Customize tabs should block a merge, not wait for a manual check.

**Next:** Keep the `wasm` check green as viewer behavior evolves.
