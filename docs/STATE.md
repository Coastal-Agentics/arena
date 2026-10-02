# STATE

_Single source of truth for Coastal Agentics (formerly Starscream Agentics). Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-10-02 ~1:20 PM ET (America/New_York)
**Phase:** 2 — Tank Arena (rules-v1 live; GATE-003 evolution M1–M2 live; M3 PettingZoo next)

## Current reality — 2026-10-02
- **GATE-002 is approved** (Sep 30 amendment): HP 460–940 across Defense 1–5; Speed sets reload (64/54/45/37/31 ticks). Rules-v1 is live (#18). **Dodging amendment approved 2026-10-02:** every scripted policy dodges (#37), and evolution's dodge-strength gene is capped at each policy's shipped value: Charger 0.65, Kiter 0.95, Sniper 0.61 (#40).
- **GATE-003 is approved** (learning tanks plan, #26). M1 evolution loop (#27) and M2 nightly job (#28) are live. M3 (Python/PettingZoo bridge) is next and owned by Shockwave.
- **ADR-014 B1 is done** (#30): generic sim core with the `Rules` trait. B2–B5 stay deferred until a second Rust game exists.
- **New evolution lineage under the dodge rules** (seed 1, from Gen 0). The pre-dodge lineage (gens 0–399; last champion Gen 399 Charger 5/3/1 at 92.92% vs the old Gen 0, held) is archived on `nightly-data` in `web/data/evolution/archive/pre-dodge-2026-10-02/` and removed from `main` (#38). First night ([run 37039026206](https://github.com/starscream-agentics/arena/actions/runs/37039026206), dispatched after #40): Gen 99 **Charger 2/5/2** wins **5,300 / 6,000 (88.3%)** vs Gen 0 on 1,000 held-out seeds, digest `d15709d4b3bd6953`. That matches Blitzwing's re-pinned M1 champion exactly. It is **held as experimental** (> 70%), and Nye is told.
- Nightly schedule moved to `37 6 * * *` (2:37 AM ET) because GitHub delayed the top-of-the-hour cron by hours (#36). The workflow was disabled during the reset and is enabled again as of 2026-10-02 1:11 PM ET.
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
| Evolution M2 (nightly) | **Live** (#28); new lineage gens 0–99 on `nightly-data` (pre-dodge gens 0–399 archived) |
| Champion promotion | **Held** — Gen 99 Charger 2/5/2 is experimental at 88.3% vs Gen 0 |
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
| Fold the new lineage into `main` as a real `nightly-data` → `main` merge PR (ADR-007), not a file copy, so the nightly's merge stays clean | CoS | when there is something to publish |

## Known engine / training limits
- Kiter mirrors draw **84.5%** of the time (informational under the 120 s cap; 72.8% before dodging).
- Kiter beats Charger **69.2%** over 400 mirrored games (76.2% before dodging); triangle draws 8.3% (< 10% target). See `games/tank/BALANCE.md` §0.
- Evolved champions still beat Gen 0 by more than 70% under the dodge cap: Blitzwing's sweep found 88.3% (seed 1) and 71.1% (seed 2), so none is promotable yet. All champions are Chargers, and nearly all sit at the cap. Pre-dodge lineage (archived): 89.6–99.9%, all held.
- Swept projectile hits treat targets as stationary within a tick.
- Placeholder Chaser/Wanderer bots remain lopsided; balance is judged on Charger, Kiter and Sniper.

## Work budget used
- 2026-09-27: Phase 0 scaffold, nightly fix; Shockwave #2/#6; Blitzwing #4.
- 2026-09-30: restructure, rules-v1, ADR-014, GATE-003 M1–M2, parity, wasm strip.
- 2026-10-01 morning: promote gens 0–199 to `main` (#34); STATE refresh.
- 2026-10-02 morning: inbox/CI check; promote gens 200–299 evolution JSON; STATE + fieldnote; dispatch skipped Oct 2 nightly. No open gates.
- 2026-10-02 midday: cron moved off the hour (#36); dodge (#37) and lineage reset (#38, `nightly-data` archive); winner-neutral viewer check (#39); dodge cap (#40); nightly re-enabled, first night of the new lineage matches the pin.
- Limits: 1 scheduled cycle per weekday + founder-triggered cycles; max 2 concurrent workers.

## Blockers
- No open gates awaiting email reply.
- Champion above 70%: do **not** make it the viewer default without a reviewed promotion PR + provenance card.
- Experimental champion under the dodge cap (88.3%): inform Nye; any further rules or cap change goes through a gate.

## Next
1. Blitzwing: viewer Gen badge and generation slider (GATE-003 M2 UI).
2. Shockwave: PettingZoo bridge (M3).
3. Nye: whether 88.3% under the cap is acceptable for now, or the field needs another change before any champion can be promoted.
