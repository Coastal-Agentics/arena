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

## ADR-007 — Nightly publishes via checked fast-forward (no new credentials)
**Status:** Accepted (2026-09-27)
**Context:** `main` requires the `lint`, `test` and `wasm` checks. The nightly job uses the default `GITHUB_TOKEN`, which cannot bypass classic branch protection, and pushes/PRs it makes do not trigger `push`/`pull_request` CI normally (bot-opened PRs get runs in an approval-required state, so bot-PR + auto-merge would stall on a human click).
**Options considered:**
1. *Ruleset with GitHub Actions as bypass actor, limited to nightly paths.* Rejected: bypass applies to the whole ruleset, not to paths, so it would let any workflow skip CI on `main`.
2. *Nightly runs the checks itself, then pushes.* Rejected as-is: checks from jobs inside the nightly run attach to the triggering commit, not the new one, so protection still refuses the push.
3. *Unprotected data branch the site reads.* Rejected: devlog entries would live off `main`, and the site would need runtime fetches from a second branch.
4. *Bot PR + auto-merge.* Rejected: stalls on required checks / workflow approval without a PAT or App.
5. *PAT or GitHub App token.* Rejected: needs a new credential (gated).
6. **Chosen: workflow_dispatch-triggered checks on a temporary branch, then fast-forward.** The nightly commits results (only `web/data/`, `docs/devlog/`; any other path fails the job), pushes the commit to `nightly/<run-id>`, dispatches `ci.yml` on it (`workflow_dispatch` is explicitly allowed from `GITHUB_TOKEN`), waits for green, then pushes that exact SHA to `main`. Protection accepts it because the required checks already passed on that commit. The temp branch is deleted afterwards.
**Trade-offs:** If `main` moves during the run, the fast-forward fails and the next night retries. Permissions: `contents: write`, `actions: write`. The CoS reviews any change to `nightly.yml` in full.
