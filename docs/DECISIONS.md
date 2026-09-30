# Architecture Decision Records

Short ADRs. Status is one of: Accepted, Proposed, Open, Superseded.

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
**Amended by ADR-014 (2026-09-30):** the move is planned in two phases (bots first, then a generic core with a `Rules` trait). Only the generic core inside `engine/` (B1) is approved; moving the tank rules into `games/tank` is deferred until a second Rust game exists. Transition re-exports, if ever needed, live in `games/tank`, never in `engine`.

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

## ADR-014 — Phase B: a generic sim core (moving the tank rules to `games/tank` deferred)
**Status:** Accepted (2026-09-30; CoS decision on internal architecture, no founder gate). Amends ADR-009: it plans the "Future work" move in two phases and fixes where any transition re-exports live. **Only step B1 is approved**; B2–B5 are deferred (see Sequencing).
**Context:** Blitzwing proposed moving the tank code out of `engine/` in two phases. **Phase A** moves the placeholder bots: Blitzwing copies `Chaser` and `Wanderer` into `games/tank` with hash-pinned tests, then an engine PR switches `engine-cli` and `engine-wasm` to those copies and deletes `engine::bots`. **Phase B**, this ADR, makes the sim core generic. What is tank-specific in `engine/` on `main` (`d4db1c5`, which includes the #16 API):
- `sim.rs`:
  - config types: `TankParams` (including `projectile_spread_still`), `TankSpawn` (including `params: Option<TankParams>`), and `MatchConfig` with `duel()` and `tank_params()`;
  - entities and results: `Tank`, `Projectile`, `Event`, `EndReason`, `Outcome`;
  - `Match`: the tank step rules (plan moves, apply them simultaneously, fire, sweep projectiles, deaths), `observe` (including `max_hp` and `los`), the team-based end check, and `state_hash`, which hashes tank and projectile fields. The same file also holds the generic parts: tick counter, seeded RNG, action history, tick limit.
- `policy.rs`: `Action`, `Observation`, `SelfObs`, `TankObs` (`max_hp`, `los`), `ProjectileObs`, `WallObs`, `MAX_OBSERVED_*`, and the `Policy` trait, which is typed on the tank `Observation` and `Action`.
- `bots.rs`: `Chaser`, `Wanderer` (Phase A).
- `replay.rs`: generic in shape (`Replay`, `ReplayPlayer`, `setup_hash`, `REPLAY_FORMAT = 4`) but typed on `Match`, `MatchConfig`, `Action` and `Outcome`. Its `FieldNotInFormat` check reads tank fields (`tanks[].params`, `params.projectile_spread_still`).
- Already generic and staying put: `angle.rs` (`Heading`, the sin/cos table, `turn_toward`), `arena.rs` (`Arena`, `Rect`, `segment_clear`, the swept tests), `json_u64.rs`, `TICK_HZ`, `DT`.

Callers today:
- `engine-cli` and `engine-wasm` import `engine::{Match, MatchConfig, …}` and `engine::bots`.
- On the unmerged `tank/rules-v1` branch (no PR yet), `games/tank` builds loadouts, policies and match specs on `engine::{Match, MatchConfig, TankParams, TankSpawn, Observation, TankObs, Action, Policy, …}`, and `engine-wasm` gains a dependency on `tank`.

**Decision:**
- **Approved now: B1 only.** `engine` gets a generic core: a `Rules` trait and `Match`, `Policy` and `Replay` generic over it. The tank rules are implemented as `TankRules` **still inside `engine`**, behind the same public names and paths.
- **Deferred: B2–B5**, the move of the tank rules into `games/tank`. They wait until a second Rust game actually exists. The next project, Saltmarsh, is Python/MuJoCo (ADR-010), so B2–B5 may never be needed.
- **End state, if B2–B5 go ahead:** `engine` keeps everything that makes a match deterministic and replayable and knows nothing about tanks, while `games/tank` (crate `tank`) implements the tank rules against the trait.

Trait sketch (**a sketch, not code on `main`**; B1 settles the exact signatures):
```rust
// engine (sketch)
pub trait Rules {
    type Config: Clone + PartialEq + Serialize + DeserializeOwned; // replayed; hashed by setup_hash
    type State: Clone;
    type Action: Copy + Default + PartialEq + Serialize + DeserializeOwned;
    type Observation;
    type Event;
    fn init(config: &Self::Config, rng: &mut MatchRng) -> Self::State;          // e.g. spawns (draws in a documented order)
    fn agents(state: &Self::State) -> usize;                                     // entries per tick in the action list
    fn is_active(state: &Self::State, agent: usize) -> bool;                     // false: policy not called, default action
    fn sanitize(action: Self::Action) -> Self::Action;                           // what is recorded (tank: clamp to [-1, 1])
    fn step(config: &Self::Config, state: &mut Self::State, actions: &[Self::Action],
            rng: &mut MatchRng, events: &mut Vec<Self::Event>);                  // one fixed tick
    fn observe(config: &Self::Config, state: &Self::State, agent: usize, tick: u32) -> Self::Observation;
    fn outcome(config: &Self::Config, state: &Self::State, tick: u32) -> Option<Outcome>; // tank keeps its order: all destroyed, last standing, tick limit
    fn hash_state(state: &Self::State, h: &mut StateHasher);                     // engine hashes the tick first
    fn check_format(config: &Self::Config, format: u32) -> Result<(), &'static str>; // today's FieldNotInFormat rule
}
pub struct Match<R: Rules> { /* config, seed, rng, tick, state, events, outcome, history: Vec<Vec<R::Action>> */ }
pub trait Policy<R: Rules> { fn act(&mut self, obs: &R::Observation) -> R::Action; }
pub struct Replay<R: Rules> { /* format, engine_version, seed, config: R::Config, actions, outcome, final_hash, setup_hash */ }
```
- **Stays in `engine`, generic:**
  - the fixed 60 Hz tick loop (`Match<R>::new`, `step`, `step_policies`, `run`), `TICK_HZ`, `DT`;
  - the single `ChaCha8Rng` (`seed_from_u64`), owned by `Match` and lent to `init`/`step` only, so each rules crate documents its own draw order and nothing else draws;
  - `state_hash` (FNV-1a: engine hashes the tick, then `hash_state`);
  - `Replay<R>`, `ReplayPlayer<R>`, `verify`, `setup_hash` (FNV-1a over the seed's LE bytes and `serde_json` of `R::Config`), `REPLAY_FORMAT`;
  - `Outcome`/`EndReason` (team winner, reasons);
  - `angle`, `arena`, `json_u64`.
- **In B1, stays in `engine`** (as `TankRules` and the existing types), and **would move to `games/tank` only in the deferred B2–B5:**
  - `TankRules`, and the config types `TankParams`, `TankSpawn` and `MatchConfig` (names kept);
  - the entities and events: `Tank`, `Projectile`, `Event`;
  - the step, observe and end rules;
  - `Action`, `Observation` and the `*Obs` types;
  - the bots (moved separately by Phase A), and the tank tests, including the hash pins.
- **Names are kept** (`MatchConfig`, `Match`, `Observation`, `Action`, and the rest) to keep churn down.
- **B1 keeps every existing path compiling.** For example, `pub type Match = generic::Match<TankRules>` (the module name is not settled), and a default type parameter on the trait, `pub trait Policy<R: Rules = TankRules>`, so `impl Policy for Chaser`, `&mut dyn Policy` and the closure impl keep working. The default-parameter pattern was checked in a scratch crate on 2026-09-30. B1 settles the exact mechanism.
- **Generics for rules, trait objects for policies**, as today: `Match<R>` is monomorphized with one instantiation (`TankRules`), so there is static dispatch and no expected size or speed cost. That is expected, not measured: B1 below must measure both. Policies stay `&mut dyn Policy<R>`. A `dyn Rules` is not an option with associated types unless the types are erased (e.g. JSON), which would cost speed and type safety.

Dependency graph, today (`main`) and after the deferred B2–B5. B1 changes only `engine`'s internals, so after B1 the graph is still the "Today" one:
```mermaid
flowchart LR
  subgraph today["Today (main)"]
    e1[engine: core + tank code + bots]
    t1[games/tank: GAME_NAME, tick_hz] --> e1
    c1[engine-cli] --> e1
    w1[engine-wasm] --> e1
    p1[web/pkg + web/arena.js] -.->|built from| w1
  end
  subgraph after["After B2-B5 (deferred)"]
    e2[engine: generic core, Rules trait]
    t2[games/tank: TankRules, tank types, bots, policies, re-exports] --> e2
    c2[engine-cli] --> t2
    c2 --> e2
    w2[engine-wasm] --> t2
    w2 --> e2
    p2[web/pkg + web/*.js] -.->|built from| w2
  end
```
(After Phase A and rules-v1, and before Phase B, `engine-cli` and `engine-wasm` already depend on `tank` as well as `engine`.)

**Where transition re-exports would live** (only relevant if B2–B5 go ahead; B1 needs none): `engine` **cannot** re-export the tank types during the transition, as Blitzwing's draft had it. `tank` depends on `engine`, so `engine` depending on `tank` is a cycle. Cargo rejects it outright, even as an optional dependency ("cyclic package dependency"; checked with two scratch crates on 2026-09-30). Every re-export points "down" the graph. Two options:
1. **In `games/tank` (recommended).** `tank` re-exports the tank types while they still live in `engine` (`pub use engine::{Match, MatchConfig, TankParams, Observation, …}`). Callers switch their imports to `tank::…`, and then the types move into `tank` behind the same paths, so callers don't change again. Later, `pub type Match = engine::Match<TankRules>` keeps `tank::Match` as the name callers use.
2. **In a thin facade crate** (e.g. `arena-tank`) that depends on `engine` and `tank` and re-exports both.

Recommendation: option 1. After Phase A, `engine-cli` and `engine-wasm` already depend on `tank`, and `tank` is where ADR-009 says the tank types belong. So its re-exports are the final paths, not a temporary shim, and callers change imports exactly once. A facade adds a crate, a graph node and a second import switch when it is removed, and saves nothing: callers must leave `engine::` paths either way. Cost: `tank` carries `pub use engine::…` lines for a while, in Blitzwing's crate.

**Determinism and compatibility (acceptance for B1, and for any later Phase B PR):**
- **Hashes byte-identical.** `documented_hashes_are_unchanged` stays green with the same constants: seed 42 (tick 447, `03722b5e86d38fac`), 7 (276, `51234f61b02b5784`, setup `0b24ce74f45e9a27`), 101 (274, `baf3fcb2cbb76c06`, setup `9cfd58498bbe3f85`) and u64::MAX (532, `f1d983e88de5d020`). When the tank tests move, the constants move unchanged.
- **Smoke run.** `engine-cli --matches 10 --seed 42` output and the 200-seed run (seeds 0–199) are byte-identical to the previous `main`.
- **`web/pkg`.** Rebuilt in every PR that touches `engine`, `engine-wasm` or `tank`; CI's drift check passes. The wasm bytes will change (code layout); the behaviour must not. Headless Chrome must give the native ticks and hashes for seeds 0, 7, 42, 43, 1234 and u64::MAX, plus a per-tank loadout.
- **Same RNG draws in the same order.** Spawns in `init` in tank id order (position candidates, then heading), spread per shot in `step` in tank id order (only when the effective spread is above 0). `state_hash` feeds the same values in the same order (tick; per tank `pos`, `vel`, `heading`, `turret`, `hp`, `cooldown`, `alive`; per projectile `owner`, `pos`, `vel`, `ttl`).
- **Serde shape of `MatchConfig` unchanged:**
  - fields `arena`, `tanks` (`team`, `pos`, `heading`, optional `params`), `params`, `max_ticks`, in that order;
  - `skip_serializing_if` on `params` and `projectile_spread_still` kept.

  Serde doesn't write type names or paths, so moving or renaming the Rust type doesn't change the JSON. Any field change would change every setup hash.
- **`REPLAY_FORMAT` stays 4 and `setup_hash` is unchanged**, because Phase B changes neither the file shape nor the sim semantics. Formats 2 and 3 keep loading, and `FieldNotInFormat` keeps working through `Rules::check_format`. A bump comes only with a real shape change.
- **Known limit, accepted:** replays don't record which game they belong to; every replay is a Tank Arena replay. Revisit only if a second game exists. That would be format 5, adding a `game` field and reading older files as tank.
- **`Outcome`/`EndReason` stay generic in `engine`** (team winner; `last_standing`, `all_destroyed`, `tick_limit`), so the replay's `outcome` doesn't change.

**Sequencing** (one PR at a time; `main` green after each). **Only B1 is approved.** It starts after rules-v1 merges (#16 already merged as `d4db1c5`). Phase A is a separate track and can land before or after B1, because B1 keeps `engine::Policy` and `engine::Observation` working.
1. **B1, Shockwave (`engine`), approved:** add `Rules`, `Match<R>`, `Policy<R>`, `Replay<R>` and a small test-only rules impl. Implement `TankRules` **inside `engine`**, keeping the existing public names and paths. No caller changes. Hash pins, smoke run and `web/pkg` checks as above; report the wasm size and native speed against `main`.
2. **B2–B5: deferred** until a second Rust game exists (may never happen: Saltmarsh is Python/MuJoCo). If they go ahead:
   - **B2, Blitzwing (`games/tank`):** add the `tank` re-exports (option 1) and switch `tank`'s own code to them. No behaviour change.
   - **B3, Shockwave (`engine-cli`, `engine-wasm`):** switch imports from `engine::` tank paths to `tank::`, and rebuild `web/pkg`.
   - **B4, Blitzwing (`games/tank`, removing the code from `engine`):** move `TankRules`, the tank types and the tank tests from `engine` into `tank`, replacing the re-exports with the real definitions. Callers don't change. Shockwave reviews the `engine` side.
   - **B5, Shockwave (docs):** update `docs/engine/`, and mark ADR-009's move as done.

**Ownership:** Shockwave (Engine Lead) owns the generic core and the `Rules` trait. Blitzwing owns the tank step rules once they live in `games/tank` (after B4). Whoever owns the rules does the moves.

**Risks:**
- **A silent determinism break.** Moving the step code can reorder float operations or RNG draws. Mitigation: move code verbatim, run the checks above in every PR, and put no behaviour changes in B1–B4.
- **A bigger wasm or a slower CLI** from generic code. Mitigation: B1 measures both against `main`.
- **Replay coupling.** `REPLAY_FORMAT` is one engine-level number while some fields it gates live in the tank config, so a tank config change needs an engine format bump. `Rules::check_format` keeps that visible.
- **Churn for Blitzwing** while the Customize tab is in flight. Mitigation: B1 starts only after rules-v1 merges and changes no caller.
- **A half-done move.** With B2–B5 deferred, the tank rules stay in `engine/` indefinitely behind a generic core, and ADR-009's end state stays unmet. Accepted: B1 alone still separates the core from the rules.
- **Scope creep** into a second game. Out of scope: Phase B adds no second `Rules`.

**Alternatives considered:**
- **Leave it as is** (ADR-009's correction stands). No risk, but every tank change stays an engine PR, the core can't carry another body (ADR-010), and ADR-009 stays unmet.
- **Generic core only** (B1 alone, `TankRules` stays in `engine`). Most of the design win and low risk, but ownership doesn't change. **Chosen for now.**
- **Move the tank code wholesale without a generic core** (each game owns its own `Match` and replay). Simplest move, but it duplicates the tick loop, RNG ownership, hashing and replay verification per game.
- **`dyn Rules` instead of generics.** Rejected above.
- **A facade crate** for re-exports (option 2). Not recommended.
- **Re-exports in `engine`.** Impossible (the crate cycle).

**Resolved (Soundwave, 2026-09-30):**
1. **Gate and timing:** no founder gate (CoS decision on internal architecture). Work starts after rules-v1 merges, not after the Customize tab.
2. **Ownership:** whoever owns the rules does the moves. Blitzwing does B4 and owns the tank step rules afterwards; Shockwave (Engine Lead) owns the generic core and the `Rules` trait.
3. **Names:** keep the existing type names (`MatchConfig`, `Match`, `Observation`, `Action`) to minimize churn.
4. **`Outcome`/`EndReason`:** stay generic in `engine`.
5. **Scope:** only B1 is approved (the generic core, with the tank rules still in `engine`). B2–B5 are deferred until a second Rust game actually exists. The next project, Saltmarsh, is Python/MuJoCo, so they may never be needed.
6. **Replays and games:** replays don't record which game they belong to. Accepted as a known limit; revisit (format 5) only if a second game exists.
