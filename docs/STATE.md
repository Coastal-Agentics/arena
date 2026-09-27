# STATE

_Single source of truth. Owned by the CoS. Updated at the end of every work cycle._

**Last updated:** 2026-09-27 (America/New_York)
**Phase:** 0 — Scaffold (CoS, alone)

## Phase 0 items
| Item | Status |
|---|---|
| Public repo `starscream-agentics/starscream` (MIT) | done |
| Cargo workspace: `engine`, `engine-cli`, `games/tank` stubs | done |
| `docs/CHARTER.md`, `DECISIONS.md`, role briefs, playbooks, first devlog | done |
| `ci.yml` (fmt, clippy, test, smoke, wasm build) | done; must stay green |
| `nightly.yml` skeleton | done (placeholder step) |
| `web/` placeholder + devlog page | done |
| Branch protection on `main` requiring CI | see blockers if not in place |
| Vercel project imported from `web/` | pending Nye |
| Test email from the CoS inbox + first `[DIGEST]` | pending |

## Open tasks
| Task | Owner | PR | Status |
|---|---|---|---|
| Import repo into Vercel (root: `web/`) | Nye | — | pending |
| Decide Vercel plan tier | Nye (gate) | — | pending |
| Send test email + first digest | CoS | — | pending |
| Phase 1: engine crate + engine-cli | Engine Lead | — | not started |
| Phase 1: `games/tank/SPEC.md` → gate | Tank Designer-Developer | — | not started |

## Open PRs
None. (Phase 0 was a bootstrap commit directly to `main`.)

## Work budget used
- 2026-09-27: 1 cycle (Phase 0 scaffold, started by Nye's message); 0 worker tasks.
- Limits: 1 scheduled cycle/day + cycles from Nye's messages; max 2 worker tasks at once.

## Blockers
- **Vercel import pending Nye.** The site is not live until Nye imports the project.
- **Vercel plan tier gate pending.** Needs Nye's decision (spending/accounts gate).
- wasm-to-Vercel pipeline is an OPEN decision (see `docs/DECISIONS.md`, ADR-005); needed before Phase 2 deploy, not before Phase 1.

## Next gate
**GATE-001** — Vercel plan tier and project import (Nye). Then **GATE-002** — Tank Arena spec (`games/tank/SPEC.md`), raised during Phase 1.
