# Architecture Decision Records

Short ADRs. Status is one of: Accepted, Open, Superseded.

## ADR-001 — Own engine, not Bevy
**Status:** Accepted (2026-09-27)
**Context:** The POC needs a tiny deterministic 2D sim that runs headless in CI and in the browser.
**Decision:** Write our own small engine crate (`engine/`). No Bevy for the POC.
**Why:** Small surface agents can hold in their heads; full control over determinism, step order and wasm size; fast CI builds.
**Cost:** We build what Bevy would give us (rendering glue, ECS-ish structure). Acceptable at POC scale.

## ADR-002 — Canvas 2D, not wgpu
**Status:** Accepted (2026-09-27)
**Decision:** The browser viewer draws with the Canvas 2D API via `web-sys`.
**Why:** Tanks, shots and HP bars need nothing more; no GPU feature detection; smaller wasm; works everywhere.
**Revisit:** If a later prototype needs thousands of sprites or shaders.

## ADR-003 — Determinism policy
**Status:** Accepted (2026-09-27)
- Fixed timestep at **60 Hz**; the sim never uses wall-clock time.
- All randomness from a seeded **`rand_chacha`** RNG passed explicitly; no global or OS RNG in the sim.
- Guarantee: **same seed → same match on the same platform**. Cross-platform bit-determinism is a stretch goal.
- **Avoid transcendental float functions** (`sin`, `cos`, `atan2`, `sqrt` where avoidable) in the sim core; prefer lookup tables or integer/fixed math.
- Deterministic iteration order (no `HashMap` iteration in sim logic).
- The headless `engine-cli` is the source of truth; CI checks that a seed reproduces a match.

## ADR-004 — Vercel instead of GitHub Pages
**Status:** Accepted (2026-09-27)
**Decision:** The public site is hosted on Vercel. Vercel's Git integration deploys `web/` on merge to `main`. No `pages.yml`.
**Why:** Nye's choice; preview deploys per PR; simple custom domain later.
**Pending:** Nye imports the project and chooses a plan tier (gated).

## ADR-005 — How the wasm build reaches Vercel
**Status:** OPEN — undecided
**Context:** The viewer needs the `engine` (and `games/tank`) compiled to wasm. Vercel's default build image has no Rust toolchain.
**Options:**
1. **Build wasm in GitHub Actions, deploy via Vercel CLI.** CI already builds wasm; reuses cache; needs a Vercel token stored as a GitHub secret (credential gate) and a deploy job in `.github/workflows/`.
2. **Build on Vercel with a Rust toolchain install.** A build command installs rustup + `wasm32-unknown-unknown` + `wasm-bindgen-cli`, then builds. No secrets in GitHub; slower builds; depends on Vercel build-time limits of the chosen plan.
**Decide by:** before the Phase 2 deploy. Needs Nye's input on credentials and plan tier.

## ADR-006 — Vercel Hobby tier for the POC
**Status:** Accepted (2026-09-27, GATE-001 approved by Nye)
**Decision:** The proof of concept runs on Vercel's **Hobby** plan. Move to **Pro** before anything commercial.
**Implications:** Hobby build-time and usage limits apply; factor them into ADR-005 (building wasm on Vercel vs. in GitHub Actions).

## ADR-007 — Nightly publishes to an unprotected `nightly-data` branch (no new credentials)
**Status:** Accepted (2026-09-27; replaces a first version of this ADR the same day)
**Context:** `main` requires the `lint`, `test` and `wasm` checks. The nightly job has only the default `GITHUB_TOKEN`: it cannot bypass classic branch protection, and pushes/PRs it makes don't trigger CI normally.
**Options considered:**
1. *Dispatch `ci.yml` on a temp branch, then fast-forward `main`.* Shipped first, then **tested and rejected**: GitHub documents that checks from `workflow_dispatch` runs never satisfy required checks, and the push was refused ("3 of 3 required status checks are expected").
2. *Ruleset with GitHub Actions as bypass actor.* Rejected: bypass can't be limited to paths, so any workflow with write access could skip CI on `main`.
3. *Bot PR + auto-merge.* Rejected: needs the "allow Actions to create PRs" setting, and CI on a `GITHUB_TOKEN` PR waits for a human to approve the run.
4. *PAT or GitHub App token.* Rejected: new credential (gated).
5. **Chosen: unprotected `nightly-data` branch.** The nightly merges `main` into `nightly-data`, commits results (only `web/data/`, `docs/devlog/`; anything else fails the job), and pushes (never force). `main` protection is untouched.
**Consequences:**
- The site reads live nightly data from `nightly-data` (e.g. `raw.githubusercontent.com/starscream-agentics/starscream/nightly-data/web/data/...`); wire this in Phase 3.
- The CoS folds `nightly-data` into `main` by a normal PR in its daily work cycle, so CI checks it and nightly devlog entries reach `main`.
- `nightly-data` holds saved data: deleting or force-pushing it is a Nye gate.
