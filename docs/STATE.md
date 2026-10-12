# STATE

_Single source of truth for Coastal Agentics. Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-10-12 ~6:45 PM ET (America/New_York)
**Phase:** 2 — Arenas (tank and racing live; the Nyborg app at /arena/play/ is the front door; nightly restarted under the V3 balance rules)

## Current reality — 2026-10-12
- **Nightly self-play was failing (Oct 8–12, 5 runs), now fixed:** every run stopped at "merge main" because #81 moved `web/data/evolution/state.json` into `archive/pre-v3-2026-10-07/` on main while `nightly-data` still carried the Oct 7 results (run 37563071167), a modify/delete conflict. Those files are byte-identical to the archive on main, so the CoS fast-forwarded `nightly-data` with a merge of main (`396b7f3`, no force push). The live lineage restarts from Gen 0 under V3; tonight's scheduled run is the check.
- **Mission (Oct 12):** "Make coastal Georgia the place where agent behavior gets designed, tested and trusted." Tagline "Good behavior, by design." Live under the hero on the [company site](https://coastal-agentics.github.io/) (site#6).
- **Nyborg app live (Oct 11):** [/arena/play/](https://coastal-agentics.github.io/arena/play/) is home: My Nyborgs + Hatch (#87), house Nyborgs and match setup for both arenas (#88), watch matches in-app with a finish card (#89), edit in-app with `customizer.html` forwarding there (#90), save a Nyborg from a match link (#91), Play first everywhere (#92). Fuzzy hair styles, Shaggy mop default, save format 2 (#86); strategy doc (#85). The [Nyborgs page](https://coastal-agentics.github.io/nyborgs/) leads into the app (nyborgs#4).
- **Tank balance V3 (Oct 7, approved by Nye):** proposal #80, fix #81 (Charger approach bounds, M1 champion re-pin, nightly restart). Pre-V3 lineage gens 0–599 archived at `web/data/evolution/archive/pre-v3-2026-10-07/` (Gen 599 Charger 3-5-1 85.78% vs Gen 0, held).
- **Naming (Oct 7):** Python package renamed `saltmarsh-arena` → `coastal-arena` (#83, CI #84, saltmarsh#6); "How the repos fit" README across all repos (#82, site#5, nyborgs#3, saltmarsh#5).
- **Team (Oct 11):** Blitzwing /nyborg owns the Nyborg agents (look, Customizer/app, behaviors, per-arena rules) and tank balance. Shockwave /engine owns the cloud robot training pipeline (Spot POC on NVIDIA Brev with Isaac Sim; tracked outside this repo) and the Arena engine.
- **Working hours rule (founder, 2026-10-03):** no agent work, commits, merges or scheduled runs on weekdays 8 AM–5 PM ET. The CoS cycle runs at 6:16 PM ET on weekdays. The Oct 7–9 cycles did not complete (outage), so no digests went out Oct 7–9.
- Live: [Nyborg app](https://coastal-agentics.github.io/arena/play/), [arena](https://coastal-agentics.github.io/arena/), [Nyborgs](https://coastal-agentics.github.io/nyborgs/), [company site](https://coastal-agentics.github.io/).

## Phase 2 status
| Item | Status |
|---|---|
| Tank rules-v1 + dodge + Customize | **Live** (#18, #37, #40); V3 approach bounds (#81) |
| Multi-game design | **Approved, built** (#43) |
| Engine M1/M2 (reward, Flat view, catalog) | **Done** (#45–#51, #59) |
| Racing v0 (R1 rules, R2 baselines, wasm) | **Live** (#57, #58, #60, #63) |
| M4 Python bridge (`coastal-arena`) | **Live** (#66; renamed #83) |
| M5 two-game viewer | **Done** (#67, #71, #72, #76) |
| Nyborg design / library | **Approved** (#44 rev 3, #49); fuzzy hair, save format 2 (#86) |
| Nyborg app (Customizer, matches, sharing) | **Live** at /arena/play/ (#85–#92) |
| Evolution nightly | **Restarted under V3**; failed Oct 8–12 on a data-branch conflict, fixed Oct 12 (`396b7f3`) |
| Champion promotion | **Held** — no champion under V3 yet |
| 3D playback (ADR-017) | **Proposed** (#79), waits on Nye |
| License | **PR #62 awaiting Nye** (plus nyborgs#1, site#1, saltmarsh#3) |
| Company website | **Live**, mission statement added (site#6) |

## Gates
| Gate | Question | Status |
|---|---|---|
| GATE-001 | Vercel plan tier | APPROVED 2026-09-27; moot (ADR-008) |
| GATE-002 | Tank Arena spec | **APPROVED 2026-09-30** |
| GATE-003 | Learning tanks plan | **APPROVED 2026-09-30** |
| Design #43 | One game system for tanks, racing, more | **APPROVED 2026-10-03** (in chat) |
| Racing v0 | racing.md | **APPROVED 2026-10-03** (in chat) |
| Nyborg design / library | #44, #49 | **APPROVED 2026-10-03** (rev 3 "for now") |
| ADR-015 | Jev as an optional tool | **Accepted** (#48) |
| Tank balance V3 | #80, #81 | **APPROVED 2026-10-07** (in chat) |

No email gate is open (inboxes checked 2026-10-12: no replies). Waiting on Nye, not gated by email: ADR-017 #79, saltmarsh ADR-003 (saltmarsh#4), license #62 and its siblings, pausing releases, the GitHub App bot identity.

## Open tasks
| Task | Owner | Status |
|---|---|---|
| Confirm tonight's nightly merges cleanly and starts Gen 0 under V3 | CoS | next cycle |
| Fold future nightly results with a **merge commit** PR (never squash) | CoS | when there is something new |
| Nyborg app polish after Nye plays with it | Blitzwing | next |
| ADR-017 3D playback (#79) | Nye | review |
| ADR-014 B2–B5 decision (second Rust game exists) | Shockwave → Nye | due |
| License #62 + nyborgs#1 + site#1 + saltmarsh#3 | Nye | review/merge (deferred) |
| Bulk `reuse annotate` SPDX headers | CoS | after #62 |
| Saltmarsh MuJoCo world (ADR-003, saltmarsh#4) | Shockwave → Nye | proposed |

## Known engine / training limits
- Pre-V3: every evolved champion was a Charger above 70% vs Gen 0 (99.35% at Gen 499, 85.78% at Gen 599), so none was promotable. V3 bounds the Charger's approach; see `docs/design/tank-balance-2026-10.md`.
- Kiter mirrors drew **84.5%** of the time pre-V3; see `games/tank/BALANCE.md` §0.
- Swept projectile hits treat targets as stationary within a tick.

## Work budget used
- 2026-09-27 → 2026-10-02: scaffold, rules-v1, ADR-014 B1, GATE-003 M1–M2, dodge + lineage reset (#2–#41).
- 2026-10-03 (Sat): multi-game build-out #43–#66.
- 2026-10-05 / 10-06 evening cycles: #67–#78 (nightly fold-ins, quiet-window guard, Customizer, re-theme).
- 2026-10-07 (after 5 PM ET, plus #80/#81 landed 7:15 AM ET): balance V3, repo map, `coastal-arena` rename (#80–#84).
- 2026-10-11 (Sun): Nyborg app M1–M7 (#85–#92, nyborgs#4).
- 2026-10-12 evening cycle: inbox/CI check (no gate replies, nothing mergeable); diagnosed 5 failed nightlies and fast-forwarded `nightly-data` (`396b7f3`); STATE refresh; digest. No background workers.
- Limits: 1 scheduled cycle per weekday at 6:16 PM ET + founder-triggered cycles, never weekdays 8 AM–5 PM ET; max 2 concurrent workers.

## Blockers
- No email gates awaiting reply.
- Champion above 70%: do **not** make it the viewer default without a reviewed promotion PR + provenance card.
- #62 is Nye's to merge; nothing else depends on it except site#1 and the SPDX follow-up.

## Next
1. Confirm tonight's nightly (first V3 generations).
2. Blitzwing: Nyborg app polish once Nye has played with it.
3. Nye: ADR-017 (#79), saltmarsh ADR-003, license #62.
4. Shockwave: ADR-014 B2–B5 recommendation; Spot POC continues off-repo.
