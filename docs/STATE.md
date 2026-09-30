# STATE

_Single source of truth for Coastal Agentics (formerly Starscream Agentics). Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-09-30 16:23 ET (America/New_York)
**Phase:** 2 — Tank Arena (the engine criteria are met; the tank spec gate remains open)

## Current reality — 2026-09-30
- The live browser viewer is [https://starscream-agentics.github.io/arena/arena.html](https://starscream-agentics.github.io/arena/arena.html) (#8). It runs the built-in Chaser/Wanderer engine matches; it is not the Tank Arena rules implementation.
- CI now rebuilds the committed `web/pkg` and fails on drift (#10). The pinned toolchain and path remapping make the wasm check reproducible.
- The `starscream-agentics` GitHub org stays the home for simulations and the arena site. A separate `coastal-agentics` org is for the company site later (#11).
- Engine documentation is in `docs/engine/` (#12). Replays are format 3 with `setup_hash`; #13 corrected the docs and ADRs and records that format 2 remains readable.
- The separate Coastal Agentics company-site draft is not published; Nye is choosing the logo before publication.
- A 3D viewer (candidate renderer: Bevy) is future work, not scheduled (ADR-012).

## Phase 2 status
| Item | Status |
|---|---|
| Tank Arena spec (`games/tank/SPEC.md`) | **GATE-002**, awaiting the founder's `APPROVE` (#4). Do not merge until approved |
| Browser viewer | **Live** at the URL above for the built-in bots (#8) |
| Committed wasm integrity | **Checked by CI**; fresh build must match `web/pkg` (#10) |
| Tank rules and three scripted policies | Blocked on GATE-002; Blitzwing implements them after approval |
| Company website | Drafted, not published; logo choice remains with Nye |

## Phase 1 exit criteria
| Criterion | Status |
|---|---|
| CI runs 10 headless matches green | **Met** (#6) |
| A seed reproduces a match | **Met** (#2) |
| `games/tank/SPEC.md` approved (GATE-002) | **Not met** (#4) |
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

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier | APPROVED 2026-09-27 (Hobby); moot since Vercel was dropped (ADR-008) |
| GATE-002 | Tank Arena spec (#4) | **OPEN.** Awaiting the founder's `APPROVE`; the PR is not to be merged before that gate |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| Tank spec gate | Blitzwing → Nye | #4 | awaiting GATE-002 `APPROVE`; PR is clean after its main-branch refresh |
| Tank rules and policies | Blitzwing | after #4 | next after GATE-002 approval |
| Saltmarsh world | CoS → Nye | — | after tank rules: MuJoCo world, then choose a walker or an arm; starting it is a gate |
| Company website publication | Nye | — | draft exists; logo choice and publication are pending |

## Known engine limits
- Swept projectile hits treat target tanks as stationary within a tick.
- Tank movement is simultaneous; a tank that would collide stays put for that tick.
- Placeholder bots are lopsided: Wanderer beat Chaser in **195/200** `engine-cli` seeds (0–199), re-measured 2026-09-30; bots are unchanged.

## Work budget used
- 2026-09-27: Phase 0 scaffold, housekeeping, nightly fix, 12:00 ET cycle; Shockwave #2 and #6; Blitzwing #4.
- 2026-09-30: restructure, viewer, CI/pkg check, org decision, engine docs/fixes; this CoS state-refresh cycle; Blitzwing's refreshed #4 remains gated.
- Limits: 1 scheduled cycle per weekday + founder-triggered cycles; max 2 concurrent workers.

## Blockers
- Tank rules and policies wait on GATE-002.
- The company site waits on Nye's logo choice and publication decision.

## Next
1. Obtain the founder's `APPROVE` on GATE-002 (#4).
2. Blitzwing implements Tank Arena rules and the three scripted policies.
3. After that, propose the Saltmarsh MuJoCo world and choose a walker or an arm.
4. Keep the Bevy 3D viewer as future, unscheduled work.
