# STATE

_Single source of truth for Coastal Agentics. Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-10-22 ~6:35 PM ET (America/New_York)
**Phase:** 2 — Arenas (tank and racing live; the Nyborg app at /arena/play/ is the front door; V3 nightly lineage live through Gen 999)

## Current reality — 2026-10-22
- **Nightly Oct 22:** run 39707543947 succeeded (gens 900–999); fold-in [#107](https://github.com/Coastal-Agentics/arena/pull/107) **merged** (merge commit). `web/data/evolution/` is on main through Gen 999. CI `test` failed once on `race_steps_allocate_only_for_history_doubling` (15 allocations over 1486 ticks, data-only PR) and passed on rerun; treat that alloc test as flaky.
- **Champion held (do not promote):** Gen 999 `sniper-5-3-1`, held-out win rate **98.55%** vs Gen 0 (5913/6000), experimental. Population 32/32 Sniper (loadouts 5-3-1 ×12, 5-2-2 ×12, 4-3-2 ×5, 4-4-1 ×2, 5-1-3 ×1).
- **Nightly Oct 21:** run 39530226205 succeeded (gens 800–899); fold-in [#105](https://github.com/Coastal-Agentics/arena/pull/105) **merged** (merge commit). `web/data/evolution/` is on main through Gen 899.
- **Previous hold:** Gen 899 `sniper-5-2-2`, held-out win rate **97.1%** vs Gen 0 (5827/6000), experimental. Population still 32/32 Sniper; plateaued around 97–99%.
- **V3 nightly continued:** Four nightly runs (Oct 17–20, all succeeded) produced gens 400–799. The Oct 19 cycle was blocked by a box network outage, so all four landed in fold-in [#103](https://github.com/Coastal-Agentics/arena/pull/103) (merge commit, not squash) **merged**; `web/data/evolution/` is on main through Gen 799.
- **Earlier hold:** Gen 799 `sniper-5-2-2`, held-out win rate **98.5%** vs Gen 0 (5910/6000). Above 70%, so it stays experimental until a reviewed promotion PR + provenance card. Population is 32/32 Sniper; the lineage has plateaued around 98–99%.
- **Compliance copy (Oct 17):** [#102](https://github.com/Coastal-Agentics/arena/pull/102) rewords public Pages/README to "agentic software development" with minimal robotics language. Green; **waits on Nye's approval** (do not merge until he says so).
- **Site domain (Oct 17):** company site is live at [coastalagentics.com](https://coastalagentics.com/).
- **Pierce (Oct 18):** onboarding the week of Oct 18 as prospective first agentic designer; MOU (Onslaught) has blanks pending from Nye.
- **Pierce onboarding (Oct 14):** Plan saved in the vault (`00 Company/03 Projects/Pierce Onboarding Plan.md`). Goal: Pierce sets up the Coastal stack himself and runs sims (including on Brev), working with Nye over Discord and a shared Obsidian vault.
- **Mission (Oct 12):** "Make coastal Georgia the place where agent behavior gets designed, tested and trusted." Tagline "Good behavior, by design." Live under the hero on the [company site](https://coastal-agentics.github.io/) (site#6).
- **Nyborg app live (Oct 11):** [/arena/play/](https://coastal-agentics.github.io/arena/play/) is home (#85–#92). The [Nyborgs page](https://coastal-agentics.github.io/nyborgs/) leads into the app (nyborgs#4).
- **Tank balance V3 (Oct 7):** #80/#81. Pre-V3 lineage gens 0–599 archived at `web/data/evolution/archive/pre-v3-2026-10-07/`.
- **Team (Oct 11):** Blitzwing /nyborg owns Nyborg agents and tank balance. Shockwave /engine owns the cloud robot training pipeline (Spot POC on NVIDIA Brev) and the Arena engine. Pierce is joining to push the robotics POC.
- **Working hours rule (founder, 2026-10-03):** no agent work, commits, merges or scheduled runs on weekdays 8 AM–5 PM ET. The CoS cycle runs at 6:16 PM ET on weekdays.
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
| Evolution nightly | **Live under V3** through Gen 999; fold-ins #94 + #96 + #98 + #100 + #103 + #105 + #107 **merged** |
| Champion promotion | **Held** — Gen 999 Sniper 5-3-1 at 98.55% vs Gen 0 (experimental) |
| 3D playback (ADR-017) | **Proposed** (#79), waits on Nye |
| License | **PR #62 awaiting Nye** (plus nyborgs#1, site#1, saltmarsh#3) |
| Company website | **Live** at coastalagentics.com, mission statement added (site#6) |
| Compliance copy (#102) | **Ready, waits on Nye** |

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

No email gate is open (inboxes checked 2026-10-22: no new replies from nyewarburton@gmail.com). Waiting on Nye, not gated by email: compliance copy #102, ADR-017 #79, saltmarsh ADR-003 (saltmarsh#4), license #62 and its siblings, pausing releases, the GitHub App bot identity.

## Open tasks
| Task | Owner | Status |
|---|---|---|
| Keep folding nightly results with a **merge commit** PR (never squash) | CoS | standing |
| Decide whether to promote / further study the held Gen 999 Sniper (98.55%); population fully Sniper and plateaued, likely needs a V4 balance look | Nye ← CoS | flagged |
| Approve compliance copy #102 | Nye | review |
| Pierce onboarding (MOU, vault copy, Grok Bot team seeds, first-week plan) | Nye + Onslaught + Pierce | week of Oct 18; MOU blanks pending from Nye |
| Nyborg app polish after Nye plays with it | Blitzwing | next |
| ADR-017 3D playback (#79) | Nye | review |
| ADR-014 B2–B5 decision (second Rust game exists) | Shockwave → Nye | due |
| License #62 + nyborgs#1 + site#1 + saltmarsh#3 | Nye | review/merge (deferred) |
| Bulk `reuse annotate` SPDX headers | CoS | after #62 |
| Saltmarsh MuJoCo world (ADR-003, saltmarsh#4) | Shockwave → Nye | proposed |
| Robotics demo / Spot POC with Pierce | Shockwave + Pierce | starting |

## Known engine / training limits
- Pre-V3: every evolved champion was a Charger above 70% vs Gen 0 (99.35% at Gen 499, 85.78% at Gen 599), so none was promotable. V3 bounds the Charger's approach; see `docs/design/tank-balance-2026-10.md`.
- V3 Gen 999: Sniper fully dominates (32/32); champion held at 98.55% vs Gen 0 (97.1% at Gen 899, 98.5% at Gen 799, 98.47% at Gen 399, was 97.6% at Gen 299, 93.3% at Gen 199, 95.63% at Gen 99).
- Kiter mirrors drew **84.5%** of the time pre-V3; see `games/tank/BALANCE.md` §0.
- Swept projectile hits treat targets as stationary within a tick.

## Work budget used
- 2026-09-27 → 2026-10-02: scaffold, rules-v1, ADR-014 B1, GATE-003 M1–M2, dodge + lineage reset (#2–#41).
- 2026-10-03 (Sat): multi-game build-out #43–#66.
- 2026-10-05 / 10-06 evening cycles: #67–#78 (nightly fold-ins, quiet-window guard, Customizer, re-theme).
- 2026-10-07 (after 5 PM ET, plus #80/#81 landed 7:15 AM ET): balance V3, repo map, `coastal-arena` rename (#80–#84).
- 2026-10-11 (Sun): Nyborg app M1–M7 (#85–#92, nyborgs#4).
- 2026-10-12 evening cycle: inbox/CI check; repaired `nightly-data` (`396b7f3`); STATE #93; digest. No background workers.
- 2026-10-13 evening cycle: inbox/CI check (no gate replies; open PRs stay with Nye); confirmed V3 nightly success; opened fold-in #94; STATE refresh; digest. No background workers (fold-in is CoS).
- 2026-10-14 evening cycle: inbox/CI check (no gate replies); confirmed V3 nightly gens 100–199; fold-in #96 merged; STATE refresh; Pierce onboarding plan noted; digest. No background workers (fold-in is CoS).
- 2026-10-15 evening cycle: inbox/CI check (no gate replies); confirmed V3 nightly gens 200–299; fold-in #98 merged; STATE refresh; digest. No background workers (fold-in is CoS).
- 2026-10-16 evening cycle: inbox/CI check (no gate replies); confirmed V3 nightly gens 300–399; fold-in #100 merged; STATE refresh; digest. No background workers (fold-in is CoS).
- 2026-10-19 evening cycle: partial. Box network outage (TLS to GitHub failed); digest only.
- 2026-10-20 evening cycle: inbox/CI check (no gate replies); four nightlies (gens 400–799) folded in via #103; STATE refresh; digest. No background workers (fold-in is CoS).
- 2026-10-21 evening cycle: inbox/CI check (no gate replies); nightly gens 800–899 folded in via #105; STATE refresh; digest. No background workers (fold-in is CoS).
- 2026-10-22 evening cycle: inbox/CI check (no gate replies); nightly gens 900–999 folded in via #107 (one flaky-test rerun); STATE refresh; digest. No background workers (fold-in is CoS; other work waits on Nye).
- Limits: 1 scheduled cycle per weekday at 6:16 PM ET + founder-triggered cycles, never weekdays 8 AM–5 PM ET; max 2 concurrent workers.

## Blockers
- No email gates awaiting reply.
- Champion above 70%: do **not** make it the viewer default without a reviewed promotion PR + provenance card. Gen 999 Sniper is in that bucket.
- #102 compliance copy waits on Nye's explicit approval.
- #62 is Nye's to merge; nothing else depends on it except site#1 and the SPDX follow-up.

## Next
0. Engine: `engine-wasm/tests/race_alloc.rs` alloc-count test flaked once on CI (Oct 22); watch for repeats before assigning.
1. Nye: approve #102 compliance copy; held Gen 999 Sniper (V4 balance look vs keep studying); Pierce MOU blanks, ADR-017 (#79), saltmarsh ADR-003, license #62.
2. Blitzwing: Nyborg app polish once Nye has played with it.
3. Shockwave + Pierce: robotics demo / Spot POC; ADR-014 B2–B5 recommendation.
