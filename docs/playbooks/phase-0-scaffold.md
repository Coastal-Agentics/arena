# Phase 0 — Scaffold

## What was set up
- Cargo workspace (`engine`, `engine-cli`, `games/tank`) with one passing test each; `rust-toolchain.toml` pins stable + rustfmt, clippy, wasm32 target.
- `engine-cli` with clap: `--matches N --seed S` → JSON. Placeholder output keeps the smoke step meaningful from day one.
- `ci.yml`: fmt, clippy `-D warnings`, test, headless smoke, `wasm32-unknown-unknown` build of `engine`.
- `nightly.yml` skeleton: schedule + manual dispatch, `contents: write`, placeholder step.
- Static `web/` placeholder for the Vercel site.
- Docs: charter, STATE, ADRs, role briefs, devlog, playbooks.
- Branch protection on `main` requiring CI (admin bypass for the CoS bootstrap).

## Reuse for the next prototype
- Copy the workspace + CI layout; add the new game as `games/<name>/`.
- Keep the wasm build check in CI from the start so the engine never drifts off wasm.
- Bootstrap commit straight to `main`, then turn on branch protection immediately.
- Record hosting and pipeline questions as OPEN ADRs rather than guessing.
