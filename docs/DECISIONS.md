# Architecture Decision Records

Short ADRs. Status is one of: Accepted, Open, Superseded.

## ADR-001 — Own engine, not Bevy
**Status:** Accepted (2026-09-27)
**Context:** The POC needs a tiny deterministic 2D sim that runs headless in CI and in the browser.
**Decision:** Write our own small engine crate (`engine/`). No Bevy for the POC.
**Why:** Small surface agents can hold in their heads; full control over determinism, step order and wasm size; fast CI builds.
**Cost:** We build what Bevy would give us (rendering glue, ECS-ish structure). Acceptable at POC scale.

## ADR-002 — Canvas 2D, not wgpu
**Status:** Accepted (2026-09-27; corrected 2026-09-30: drawn from plain JavaScript, not `web-sys`)
**Decision:** The browser viewer draws with the Canvas 2D API via `web-sys`.
**Why:** Tanks, shots and HP bars need nothing more; no GPU feature detection; smaller wasm; works everywhere.
**Revisit:** If a later prototype needs thousands of sprites or shaders.
**Correction (2026-09-30):** The viewer as built (#8) draws with the Canvas 2D API from plain JavaScript (`web/arena.js`); `web-sys` is not used. The wasm module (`engine-wasm`) only runs the sim and hands the state to JS as JSON. Canvas 2D instead of wgpu stands. See `docs/engine/wasm-and-web.md`.

## ADR-003 — Determinism policy
**Status:** Accepted (2026-09-27)
- Fixed timestep at **60 Hz**; the sim never uses wall-clock time.
- All randomness from a seeded **`rand_chacha`** RNG passed explicitly; no global or OS RNG in the sim.
- Guarantee: **same seed → same match on the same platform**. Cross-platform bit-determinism is a stretch goal.
- **Avoid transcendental float functions** (`sin`, `cos`, `atan2`, `sqrt` where avoidable) in the sim core; prefer lookup tables or integer/fixed math.
- Deterministic iteration order (no `HashMap` iteration in sim logic).
- The headless `engine-cli` is the source of truth; CI checks that a seed reproduces a match.

## ADR-004 — Vercel instead of GitHub Pages
**Status:** Superseded by ADR-008 (2026-09-30)
**Decision:** The public site is hosted on Vercel. Vercel's Git integration deploys `web/` on merge to `main`. No `pages.yml`.
**Why:** Nye's choice; preview deploys per PR; simple custom domain later.
**Pending:** Nye imports the project and chooses a plan tier (gated).

## ADR-005 — How the wasm build reaches Vercel
**Status:** Superseded by ADR-008 (2026-09-30), which answers it: wasm is committed under `web/pkg` and deployed as static files
**Context:** The viewer needs the `engine` (and `games/tank`) compiled to wasm. Vercel's default build image has no Rust toolchain.
**Options:**
1. **Build wasm in GitHub Actions, deploy via Vercel CLI.** CI already builds wasm; reuses cache; needs a Vercel token stored as a GitHub secret (credential gate) and a deploy job in `.github/workflows/`.
2. **Build on Vercel with a Rust toolchain install.** A build command installs rustup + `wasm32-unknown-unknown` + `wasm-bindgen-cli`, then builds. No secrets in GitHub; slower builds; depends on Vercel build-time limits of the chosen plan.
**Decide by:** before the Phase 2 deploy. Needs Nye's input on credentials and plan tier.

## ADR-006 — Vercel Hobby tier for the POC
**Status:** Superseded by ADR-008 (2026-09-30); Vercel is dropped, so GATE-001's plan tier no longer applies
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
5. **Chosen: unprotected `nightly-data` branch.** The nightly merges `main` into `nightly-data`, commits results (only `web/data/`, `docs/fieldnotes/` (was `docs/devlog/` before ADR-011); anything else fails the job), and pushes (never force). `main` protection is untouched.
**Consequences:**
- The site reads live nightly data from `nightly-data` (e.g. `raw.githubusercontent.com/starscream-agentics/arena/nightly-data/web/data/...`); wire this in Phase 3.
- The CoS folds `nightly-data` into `main` by a normal PR in its daily work cycle, so CI checks it and nightly devlog entries reach `main`.
- `nightly-data` holds saved data: deleting or force-pushing it is a Nye gate.

## ADR-008 — GitHub Pages instead of Vercel; wasm committed under `web/pkg`
**Status:** Accepted (2026-09-30, Nye approved the restructure). Supersedes ADR-004 and ADR-006; answers ADR-005.
**Decision:** The public site is GitHub Pages, deployed by `.github/workflows/pages.yml` on push to `main` using the official actions (`configure-pages`, `upload-pages-artifact`, `deploy-pages`). `web/` is uploaded as-is: no build step. The wasm build and its JS glue are committed under `web/pkg` and served as static files. `web/vercel.json` is removed.
**Why:** No new account, token or plan tier; the deploy lives next to CI where the CoS owns it; the repo stays self-contained. Answers ADR-005 without a secret or a Rust toolchain on a host.
**Cost:** No per-PR preview deploys. Committed wasm must be rebuilt in the same PR as engine changes; the Engine Lead's brief says so.
**URL:** `https://starscream-agentics.github.io/arena/` (stays there; ADR-013).

## ADR-009 — The engine core is generic; tank specifics live in `games/tank`
**Status:** Accepted (2026-09-30; corrected 2026-09-30: this is the target layout, not yet the code)
**Decision:** `engine/` holds only the generic sim: fixed step, seeded RNG, arena, entities, match lifecycle, replay, the `Policy` trait. Tank rules, tank observations/actions and tank policies live in `games/tank`. New tank features never land in `engine/`.
**Why:** Coastal Agentics trains robots; the same core should later carry other bodies. Keeping game logic out of the core keeps it small and reusable.
**Correction (2026-09-30):** The code does not match this yet. Today the tank specifics live in `engine/`: the `Tank` and `Projectile` entities, `TankParams`, `TankSpawn`, `MatchConfig::duel` and the tank step rules (driving, turret, firing, hits) in `sim.rs`; the tank `Observation`/`Action` pair, with the `Policy` trait typed on them, in `policy.rs`; and the placeholder policies `Chaser` and `Wanderer` in `bots.rs`. `games/tank` is a stub (`GAME_NAME`, `tick_hz()`) waiting on the Tank Arena spec (GATE-002). See `docs/engine/`.
**Future work:** Move the tank-specific code from `engine/` into `games/tank` so `engine/` is the generic core this ADR describes. Not scheduled; no code has moved.

## ADR-010 — The Rust core is a candidate browser viewer for Saltmarsh
**Status:** Open (candidate, 2026-09-30)
**Context:** The second project is a Saltmarsh world (MuJoCo, Python). Training and physics stay in Python/MuJoCo.
**Proposal:** Reuse the Rust core compiled to wasm to play back Saltmarsh replays or logged states in the browser, so the site can show trained behavior without a Python runtime.
**Decide by:** when Saltmarsh is proposed to Nye as a gate. Alternative: a MuJoCo-native web viewer.

## ADR-011 — Renamed to Coastal Agentics
**Status:** Accepted (2026-09-30)
**Decision:** Starscream Agentics is now **Coastal Agentics** (Savannah, Georgia): "We train robots, with open tools, on the Georgia coast." Founded on GitHub October 1, 2026. The repo `starscream` is renamed `arena` (GitHub redirects the old URL); the `starscream-agentics` org keeps its name (ADR-013). The devlog is now field notes (`docs/fieldnotes/`, `web/fieldnotes.html`; `web/devlog.html` redirects). Email prefixes are `[COASTAL]`. Crate names are unchanged.

## ADR-012 — Tank demo stays 2D canvas; 3D environment is future work
**Status:** Accepted for the 2D demo (founder decision, 2026-09-30). The 3D work is future work, **not scheduled**.
**Decision:** The Tank Arena demo stays a 2D canvas viewer (ADR-002). No 3D work in the tank POC.
**Future work:** We will need a 3D environment for objects moving in space. Candidate renderer: **Bevy**, reading the same match data (replays/state logs) the engine already produces, so the sim stays unchanged. It could become a shared viewer for Saltmarsh/MuJoCo worlds too (see ADR-010). Revisits ADR-001 for rendering only, not for the sim core.
**Decide by:** when a project needs 3D; the CoS proposes it to Nye as a gate then.

## ADR-013 — `starscream-agentics` stays; the company site gets its own org later
**Status:** Accepted (founder decision, 2026-09-30). Replaces the planned org rename in ADR-011.
**Decision:** The `starscream-agentics` GitHub org is **not** renamed. It stays the home for simulations; this repo and its site remain at `https://starscream-agentics.github.io/arena/`. A separate `coastal-agentics` org will host the company site at `https://coastal-agentics.github.io` later (not scheduled).
**Consequences:** No URL change for the arena site, the repo or `nightly-data` links. The "rename org" task and blocker are closed.
