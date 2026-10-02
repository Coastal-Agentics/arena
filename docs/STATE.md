# STATE

_Single source of truth for Coastal Agentics (formerly Starscream Agentics). Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-10-02 ~09:30 ET (America/New_York)
**Phase:** 2 — Tank Arena (rules-v1 live; GATE-003 evolution M1–M2 live; M3 PettingZoo next)

## Current reality — 2026-10-02
- **GATE-002 is approved** (Sep 30 amendment): HP 460–940 across Defense 1–5; Speed sets reload (64/54/45/37/31 ticks). Rules-v1 is live (#18).
- **GATE-003 is approved** (learning tanks plan, #26). M1 evolution loop (#27) and M2 nightly job (#28) are live. M3 (Python/PettingZoo bridge) is next and owned by Shockwave.
- **ADR-014 B1 is done** (#30): generic sim core with the `Rules` trait. B2–B5 stay deferred until a second Rust game exists.
- **Evolution lineage reset 2026-10-02** after #37 (scripted dodge) changed the gene tables. The pre-dodge lineage (seed 1, gens 0–399; last champion Gen 399 Charger 5/3/1 at 92.92% vs the old Gen 0, held as experimental) is archived on `nightly-data` in `web/data/evolution/archive/pre-dodge-2026-10-02/`. Its copy on `main` is removed. The next nightly starts a fresh Gen 0 under the new rules, after Blitzwing's dodge-strength cap PR lands.
- The Oct 2 3 AM ET scheduled nightly appears skipped again; this morning cycle dispatched `workflow_dispatch` to catch up ([run 37012871253](https://github.com/starscream-agentics/arena/actions/runs/37012871253)).
- Parity and packaging: native-vs-wasm check (#31, #32); wasm strip (#33). Runner pinned to `ubuntu-24.04` (#29).
- Live viewer: [arena](https://starscream-agentics.github.io/arena/arena.html). Company site live: [coastal-agentics.github.io](https://coastal-agentics.github.io/) (headline v1.2). Sims stay under `starscream-agentics`.

## Phase 2 status
| Item | Status |
|---|---|
| Tank Arena spec | **Approved — GATE-002** |
| Rules-v1 + three policies + Customize | **Live** (#18) |
| ADR-014 B1 generic core | **Done** (#30) |
| GATE-003 plan | **Approved** (#26) |
| Evolution M1 (Rust GA) | **Live** (#27) |
| Evolution M2 (nightly) | **Live** (#28); lineage reset to Gen 0 after #37 (pre-dodge gens 0–399 archived on `nightly-data`) |
| Champion promotion | None yet under the new rules (pre-dodge champions were all held as experimental) |
| Viewer Gen badge / slider (M2 UI) | **Not started** (Blitzwing) |
| PettingZoo bridge (M3) | **Not started** (Shockwave) |
| Company website | **Live** at coastal-agentics.github.io |

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier | APPROVED 2026-09-27; moot (ADR-008) |
| GATE-002 | Tank Arena spec | **APPROVED 2026-09-30** |
| GATE-003 | Learning tanks plan | **APPROVED 2026-09-30** |

## Open tasks
| Task | Owner | Status |
|---|---|---|
| Viewer Gen badge + generation slider | Blitzwing | next |
| PettingZoo / `engine-py` bridge (M3) | Shockwave | after M2 UI or in parallel when free |
| Saltmarsh MuJoCo world | CoS → Nye | after M3; starting it is a gate |
| Diversity / anti-plateau (scripted dodge or rules) | Blitzwing + CoS | needs Nye call if rules change |

## Known engine / training limits
- Kiter mirrors draw **72.8%** of the time (informational under the 120 s cap).
- Kiter beats Charger **76.2%** over 400 mirrored games (near 80% ceiling).
- Pre-dodge lineage (archived): champions won 89.6–99.9% vs the old Gen 0, all held as experimental; the population collapsed to Chargers by ~gen 100. The new lineage starts after the dodge-strength cap.
- Swept projectile hits treat targets as stationary within a tick.
- Placeholder Chaser/Wanderer bots remain lopsided; balance is judged on Charger, Kiter and Sniper.

## Work budget used
- 2026-09-27: Phase 0 scaffold, nightly fix; Shockwave #2/#6; Blitzwing #4.
- 2026-09-30: restructure, rules-v1, ADR-014, GATE-003 M1–M2, parity, wasm strip.
- 2026-10-01 morning: promote gens 0–199 to `main` (#34); STATE refresh.
- 2026-10-02 morning: inbox/CI check; promote gens 200–299 evolution JSON; STATE + fieldnote; dispatch skipped Oct 2 nightly. No open gates.
- Limits: 1 scheduled cycle per weekday + founder-triggered cycles; max 2 concurrent workers.

## Blockers
- No open gates awaiting email reply.
- Champion above 70%: do **not** make it the viewer default without a reviewed promotion PR + provenance card.
- Evolution plateau / all-Charger mix: inform Nye; no rules change without a gate.

## Next
1. Blitzwing: viewer Gen badge and generation slider (GATE-003 M2 UI).
2. Shockwave: PettingZoo bridge (M3).
3. Decide with Nye whether plateau needs a rules/dodge change or waits for RL.
