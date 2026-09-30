# STATE

_Single source of truth for Coastal Agentics (formerly Starscream Agentics). Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-09-30 18:25 ET (America/New_York)
**Phase:** 2 — Tank Arena (rules-v1 is live; ADR-014 B1 is in progress)

## Current reality — 2026-09-30
- **GATE-002 is approved**, including the Sep 30 amendment: HP is 460–940 across Defense levels 1–5, and Speed sets reload (64/54/45/37/31 ticks).
- **Tank Arena rules-v1 is live (#18):** every tank has a 9-point Attack/Speed/Defense loadout; the Charger, Kiter and Sniper policies are implemented; and the viewer's Customize tab supports shareable links.
- The Watch-tab fix is in (#20), and the headless viewer browser check runs in CI (#21). The live viewer is [https://starscream-agentics.github.io/arena/arena.html](https://starscream-agentics.github.io/arena/arena.html).
- **ADR-014 is accepted (#17):** Phase A is done (#19, #22), and B1, the generic engine core, is in progress. B2–B5 remain deferred until a second Rust game exists.
- The `starscream-agentics` GitHub org stays the home for simulations and the arena site. A separate `coastal-agentics` org is for the company site later (#11).
- Engine documentation is in `docs/engine/` (#12). Replays are format 4 with `setup_hash`; #13 corrected the docs and ADRs and records that format 2 remains readable.
- The separate Coastal Agentics company-site draft is not published; Nye is choosing the logo before publication.
- A 3D viewer (candidate renderer: Bevy) is future work, not scheduled (ADR-012).

## Phase 2 status
| Item | Status |
|---|---|
| Tank Arena spec (`games/tank/SPEC.md`) | **Approved — GATE-002**, including the Sep 30 amendment |
| Tank rules and three scripted policies | **Live** in rules-v1 (#18): 9-point loadouts; Charger, Kiter and Sniper |
| Browser viewer | **Live** with Watch and Customize tabs; links share seed, behaviors and loadouts (#18) |
| Watch-tab correctness | **Fixed** (#20) |
| Browser regression check | **Checked by CI** in the `wasm` job (#21) |
| ADR-014 Phase A | **Done** (#19, #22) |
| ADR-014 B1 generic core | **In progress** (#17) |
| Company website | Drafted, not published; logo choice remains with Nye |

## Phase 1 exit criteria
| Criterion | Status |
|---|---|
| CI runs 10 headless matches green | **Met** (#6) |
| A seed reproduces a match | **Met** (#2) |
| `games/tank/SPEC.md` approved (GATE-002) | **Met** (#18) |
| Phase 1 playbook entry | **Not met** |

## Merged
| PR | What |
|---|---|
| #1 | CoS housekeeping; GATE-001 (Vercel Hobby, superseded by ADR-008) |
| #2 | Phase 1 engine core + `engine-cli`, deterministic |
| #5 | Nightly publishes to the unprotected `nightly-data` branch (ADR-007) |
| #6 | Swept projectile hits, seeds as strings, simultaneous movement, `REPLAY_FORMAT` 2 |
| #7 | CoS cycle 2026-09-27: STATE, roster names, cards |
| #8 | Live wasm viewer at `web/arena.html` |
| #9 | Coastal Agentics restructure, GitHub Pages, field notes, provenance cards |
| #10 | CI rebuild-and-compare for committed `web/pkg`; favicon; ADR-012 |
| #11 | `starscream-agentics` org stays; company site gets a separate org later |
| #12 | Engine documentation and rustdoc in `docs/engine/` |
| #13 | Replay `REPLAY_FORMAT` 3 with `setup_hash`; doc and ADR corrections |
| #16 | Engine support for Tank Arena: LOS, per-tank params, max HP and stationary accuracy |
| #17 | ADR-014 accepted; B1 generic engine core approved |
| #18 | Tank Arena rules-v1: loadouts, policies, Customize tab and balance retune |
| #19 | Phase A, step 1: copy placeholder bots into `games/tank` |
| #20 | Watch tab always describes the running match |
| #21 | Headless viewer browser check in CI |
| #22 | Phase A, step 2: callers use `tank::{Chaser, Wanderer}`; engine bots removed |

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier | APPROVED 2026-09-27 (Hobby); moot since Vercel was dropped (ADR-008) |
| GATE-002 | Tank Arena spec (#4) | **APPROVED 2026-09-30**, including the Sep 30 amendment in #18: HP 460–940 and Speed-set reload |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| ADR-014 B1 generic engine core | Shockwave | #17 | in progress; preserve hashes, replays and the `wasm` check |
| Saltmarsh world | CoS → Nye | — | after B1: MuJoCo world, then choose a walker or an arm; starting it is a gate |
| Company website publication | Nye | — | draft exists; logo choice and publication are pending |

## Known engine limits
- Kiter mirrors draw **72.8%** of the time (the 120-second cap makes these draws informational, not a policy target).
- Kiter beats Charger **76.2%** over 400 mirrored games, near the **80% ceiling**; the full triangle remains within its 55–80% target.
- A default 3/3/3 kill takes **33 hits**.
- Swept projectile hits treat target tanks as stationary within a tick.
- Tank movement is simultaneous; a tank that would collide stays put for that tick.
- Placeholder Chaser/Wanderer bots remain lopsided; Tank Arena balance is judged on Charger, Kiter and Sniper.

## Work budget used
- 2026-09-27: Phase 0 scaffold, housekeeping, nightly fix, 12:00 ET cycle; Shockwave #2 and #6; Blitzwing #4.
- 2026-09-30: restructure, viewer, CI/pkg check, org decision, engine docs/fixes, rules-v1, ADR-014, Phase A, Watch fix and browser CI check; this state/rules-live refresh.
- Limits: 1 scheduled cycle per weekday + founder-triggered cycles; max 2 concurrent workers.

## Blockers
- No tank-rules gate is open. B1 is the active architecture task.
- The company site waits on Nye's logo choice and publication decision.

## Next
1. Complete ADR-014 B1, the generic engine core.
2. Propose the Saltmarsh MuJoCo world and choose a walker or an arm.
3. Keep the Bevy 3D viewer as future, unscheduled work.
