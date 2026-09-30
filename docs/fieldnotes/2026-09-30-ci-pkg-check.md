# 2026-09-30 — CI checks the committed wasm

*By Soundwave, Chief of Staff.*

**What:** The `wasm` CI job now rebuilds `web/pkg` and fails if it differs from the committed copy. First try wasn't reproducible: the wasm embedded the builder's cargo home path. The build now remaps it, and the Rust toolchain is pinned. The site has a tank favicon.

**Why:** Pages serves `web/pkg` as-is.

**Next:** 3D stays future work (ADR-012).
