# STATE

_Single source of truth. Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-09-27 12:00 ET (America/New_York)
**Phase:** 1 — Engine + Tank spec (started 2026-09-27)

## Phase 0 — done
Repo, workspace stubs, CI (`lint`, `test`, `wasm`), nightly workflow, docs, web placeholder, branch protection on `main` requiring CI. Playbook: `docs/playbooks/phase-0-scaffold.md`.

## Phase 1 exit criteria (CHARTER: "Done when CI runs 10 headless matches green and a seed reproduces a match")
| Criterion | Status |
|---|---|
| CI runs 10 headless matches green | **Met.** The `test` job runs `engine-cli --matches 10 --seed 42`; green on `main` after #6 |
| A seed reproduces a match | **Met.** Same seed gives identical output (verified in #2; `engine-cli` tests `same_seed_same_json` and `match_i_is_reproducible_alone` run in CI) |
| `games/tank/SPEC.md` approved (GATE-002) | **Not met.** In revision (#4) |
| Phase 1 playbook entry in `docs/playbooks/` | **Not met.** Written when Phase 1 closes |

The engine criteria are met. Phase 1 stays open until GATE-002 clears and the playbook entry exists.

## Merged
| PR | What |
|---|---|
| #1 | CoS housekeeping; GATE-001 approved: Vercel Hobby for the POC (ADR-006) |
| #2 | Phase 1 engine core + `engine-cli`, deterministic; verified same seed gives identical output |
| #5 | Nightly publishes to the unprotected `nightly-data` branch (ADR-007 revised); the CoS copies results to `main` by PR in its daily cycle; deleting or force-pushing `nightly-data` is gated |
| #6 | Swept projectile hits, seeds as strings, simultaneous movement, `REPLAY_FORMAT` 2 |

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier for the POC | **APPROVED** by Nye 2026-09-27: Hobby for the POC; move to Pro before anything commercial (ADR-006) |
| GATE-002 | Tank Arena spec (`games/tank/SPEC.md`, PR #4) | **IN REVISION.** Nye answered the questions: a match is a draw at the time cap; one tank type with adjustable attack, speed and defense stats. Blitzwing is revising; then it goes back to Nye for `APPROVE` |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| Revise `games/tank/SPEC.md` per Nye's GATE-002 answers | Blitzwing (Tank Designer-Developer) | #4 | in revision |
| Import repo into Vercel (Hobby, root `web/`) | Nye | — | pending Nye |
| Verify the AgentMail inbox (optional) | Nye | — | pending Nye, optional |
| Decide wasm-to-Vercel pipeline (ADR-005) | Soundwave → Nye | — | open decision; needed before Phase 2 deploy |

## Open PRs
- #4 — GATE-002 draft: Tank Arena `SPEC.md` (Blitzwing). Do not merge until Nye approves.

## Known engine limits
- The swept hit check treats target tanks as stationary within a tick (the projectile's path is swept; the target's motion during that tick is not).

## Work budget used
- 2026-09-27: cycles for Phase 0 scaffold, housekeeping, nightly fix, and this 12:00 ET cycle, all started by Nye's messages. Worker tasks: Shockwave (engine core #2, follow-ups #6) done; Blitzwing (spec #4) in revision.
- Limits: 1 scheduled cycle/day + cycles from Nye's messages; max 2 worker tasks at once.

## Blockers
- **Vercel import pending Nye.** The site is not live until the project is imported.
- wasm-to-Vercel pipeline is OPEN (ADR-005); not blocking Phase 1, blocks the Phase 2 deploy.

## Next
1. **GATE-002**: Blitzwing's revised spec goes back to Nye for `APPROVE`.
2. After approval: implement the tank game (charger / kiter / sniper policies), a line-of-sight helper, adjustable tank stats (attack, speed, defense), and the wasm browser viewer.
3. Write the Phase 1 playbook entry and close Phase 1.
