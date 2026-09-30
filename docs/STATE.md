# STATE

_Single source of truth for Coastal Agentics (formerly Starscream Agentics). Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-09-30 15:50 ET (America/New_York)
**Phase:** 2 — Tank Arena (engine criteria of Phase 1 met; spec gate and Phase 1 playbook still open)

## Restructure — 2026-09-30 (ADR-011)
- Company renamed **Coastal Agentics**, Savannah, Georgia. Founded on GitHub October 1, 2026. Charter replaced (`docs/CHARTER.md`).
- Repo renamed `starscream` → **`arena`**; old URLs redirect. Org profile updated (name, description, location).
- Org: `starscream-agentics` is **not** renamed; it stays the home for simulations (ADR-013). A separate `coastal-agentics` org will host the company site later.
- Hosting moved from Vercel to **GitHub Pages** (ADR-008): `.github/workflows/pages.yml`, site at https://starscream-agentics.github.io/arena/ (stays there, ADR-013). `web/vercel.json` removed.
- Devlog is now **field notes**: `docs/fieldnotes/`, `web/fieldnotes.html` (`web/devlog.html` redirects). Role briefs rebranded in `docs/roles/`. Provenance card template: `docs/CARD.md`.

## Phase 2 status
| Item | Status |
|---|---|
| Tank Arena spec (`games/tank/SPEC.md`) | At **GATE-002**, awaiting Nye (#4). Do not merge until approved |
| Browser viewer (wasm) | **In progress**: Shockwave, branch `engine/viewer-v0` (#8). Needs a rebase onto the rename (field notes paths, `fieldnotes.html`) |
| Pages deploy | **Workflow added** (`pages.yml`), deploys `web/` on push to `main` |
| Tank rules, three scripted policies | Blocked on GATE-002 |

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
| #1 | CoS housekeeping; GATE-001 (Vercel Hobby, now superseded by ADR-008) |
| #2 | Phase 1 engine core + `engine-cli`, deterministic |
| #5 | Nightly publishes to the unprotected `nightly-data` branch (ADR-007) |
| #6 | Swept projectile hits, seeds as strings, simultaneous movement, `REPLAY_FORMAT` 2 |
| #7 | CoS cycle 2026-09-27: STATE, roster names, cards |
| this PR | Coastal Agentics restructure (charter, rename, Pages, field notes, CARD.md) |

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier | APPROVED 2026-09-27 (Hobby); **moot** since Vercel was dropped (ADR-008) |
| GATE-002 | Tank Arena spec (#4) | **OPEN.** Revised per Nye's answers (draw at time cap; one tank type with adjustable attack, speed, defense); awaiting `APPROVE` |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| Tank spec gate | Blitzwing → Nye | #4 | awaiting GATE-002 |
| Viewer v0 | Shockwave | #8 | in progress |
| Verify the AgentMail inbox (optional) | Nye | — | optional |

## Known engine limits
- The swept hit check treats target tanks as stationary within a tick.

## Work budget used
- 2026-09-27: Phase 0 scaffold, housekeeping, nightly fix, 12:00 ET cycle (all Nye-triggered). Shockwave #2, #6; Blitzwing #4.
- 2026-09-30: restructure cycle (Nye-triggered); Shockwave viewer #8 running (1 of 2 worker slots).
- Limits: 1 scheduled cycle per weekday + founder-triggered cycles; max 2 concurrent workers.

## Blockers
- Tank rules wait on GATE-002.

## Next
1. GATE-002 approval, then tank rules and the three scripted policies.
2. Merge the viewer (#8) once rebased and green; confirm it on the Pages site.
3. Write the Phase 1 playbook entry and close Phase 1.
4. First `[COASTAL][MONTHLY]` roll-up to Nye on 2026-10-01.
