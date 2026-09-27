# STATE

_Single source of truth. Owned by the CoS. Updated at the end of every work cycle._

**Last updated:** 2026-09-27 (America/New_York)
**Phase:** 1 — Engine + Tank spec (started 2026-09-27)

## Phase 0 — done
Repo, workspace stubs, CI (`lint`, `test`, `wasm`), nightly workflow, docs, web placeholder, branch protection on `main` requiring CI. Playbook: `docs/playbooks/phase-0-scaffold.md`.

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier for the POC | **APPROVED** by Nye 2026-09-27: Hobby for the POC; move to Pro before anything commercial (ADR-006) |
| GATE-002 | Tank Arena spec (`games/tank/SPEC.md`) | not yet raised; waiting on the draft |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| Engine core + engine-cli | Engine Lead | — | in progress |
| `games/tank/SPEC.md` draft → GATE-002 | Tank Designer-Developer | — | queued |
| Import repo into Vercel (Hobby, root `web/`) | Nye | — | pending |
| Send test email + first digest | CoS | — | pending |
| Decide wasm-to-Vercel pipeline (ADR-005) | CoS → Nye | — | open; needed before Phase 2 deploy |

## Open PRs
- CoS housekeeping: STATE/ADR updates, nightly publish path (ADR-007), checkout v7, devlog.

## Work budget used
- 2026-09-27: 2 cycles (Phase 0 scaffold; housekeeping), both started by Nye's messages; worker tasks: 1 running (Engine Lead), 1 queued (Tank Designer-Developer).
- Limits: 1 scheduled cycle/day + cycles from Nye's messages; max 2 worker tasks at once.

## Blockers
- **Vercel import pending Nye.** The site is not live until the project is imported.
- wasm-to-Vercel pipeline is OPEN (ADR-005); not blocking Phase 1.

## Next gate
**GATE-002** — Tank Arena spec, raised when the Tank Designer-Developer's draft is ready.
