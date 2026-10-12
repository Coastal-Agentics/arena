# STATE

_Single source of truth for Coastal Agentics. Owned by the CoS (Soundwave). Updated at the end of every work cycle._

**Last updated:** 2026-10-06 ~6:35 PM ET (America/New_York)
**Phase:** 2 — Arenas (tank and racing live in the viewer; Python bridge M4 live; M5 slice done; Customizer first slice live)

## Current reality — 2026-10-06
- **Oct 6 (after 5 PM ET):** Nyborg Customizer first slice live at [customizer](https://coastal-agentics.github.io/arena/customizer.html) (#73): library, look editor, per-game builds through the engine's validator, export/import. **Race these** (#76): pick 2–4 Nyborgs and race them on the Ring with their own looks; tank links carry looks too. Coastal re-theme (#74, ADR-016) and public copy pass (#75). Racing in the browser engine and viewer landed Oct 5 (#71, #72).
- **Nightly Oct 6 (run 37408275899, 11:17 PM ET Oct 5, outside the quiet window, still ~3.7 h late):** gens 400–499. Gen 499 **Charger 3-4-2** wins **5,961 / 6,000 (99.35%)** vs Gen 0, digest `623218a3ca642a64` — **held as experimental**. Fold-in PR **#77** (merge commit).
- **Working hours rule (founder, 2026-10-03):** no agent work, commits, merges or scheduled runs on weekdays 8 AM–5 PM ET. The CoS cycle runs at 6:16 PM ET on weekdays. The nightly's scheduled runs were landing 5–9 hours late (Oct 5 started 11:11 AM ET, inside the window); **#69 (merged)** moves it to 7:37 PM ET and adds a hard weekday 7 AM–5 PM ET skip; tonight's run is the first on the new slot.
- **Multi-game design approved and built (2026-10-03):** design #43; engine M1/M2 (#45, #46, #48, #50); per-game build catalog (#51, #59); M3a replay format 5 (#56); **racing v0** rules R1 (#57) and baselines R2 (#58), catalog (#60), in wasm GAMES (#63, #64); **M4** `coastal-arena` Python bridge for tank and racing (#66). ADR-015 (Jev as an optional tool) accepted (#47, #48).
- **Nyborgs:** character design rev 3 settled for now (#44: cream head, dot eyes, no mouth, 2–4 yarn strands, Yarn Red default; cosmetics never affect stats). Library and save format approved (#49, #55). **M5 first slice live:** Nyborgs are the default tank-viewer sprites, `?sprites=classic` falls back (#67). The real Customizer comes next.
- **Evolution (dodge lineage, seed 1):** nightly gens 0–399 on `nightly-data`. Gen 399 **Charger 2-4-3** wins **5,057 / 6,000 (84.28%)** vs Gen 0 on 1,000 held-out seeds, digest `5e4b79cb74a97f07` — **held as experimental**. **#68 (merged, `75d0248`)** folded it (plus the pre-dodge archive) into `main` as a real merge commit (ADR-007).
- **Licensing:** #62 (MIT OR Apache-2.0, holder Nye Warburton, Nyborg art all rights reserved, DCO + REUSE checks) is green and **waits on Nye to review and merge**, together with Coastal-Agentics/nyborgs#1 and Coastal-Agentics/coastal-agentics.github.io#1 (privacy/terms pages), or the footer links 404.
- Field notes now render from a generated index (#52–#54). Parity fixtures refreshed (#61); wasm slimmed to 302,776 B (#59) and the build script hardened (#65).
- **Moved into Coastal-Agentics (2026-10-06, ADR-016):** `arena` and the Nyborgs landing page (`nyborgs`, formerly the old org's splash repo) now live in the Coastal-Agentics org. The internal codename is retired from public pages, READMEs and repo metadata; dated field notes and past ADRs keep their wording as history. All three sites share one header, footer and theme.
- Live: [arena](https://coastal-agentics.github.io/arena/) ([viewer](https://coastal-agentics.github.io/arena/arena.html)), [Nyborgs](https://coastal-agentics.github.io/nyborgs/), [company site](https://coastal-agentics.github.io/).

## Phase 2 status
| Item | Status |
|---|---|
| Tank rules-v1 + dodge + Customize | **Live** (#18, #37, #40) |
| Multi-game design | **Approved, built** (#43) |
| Engine M1/M2 (reward, Flat view, catalog) | **Done** (#45–#51, #59) |
| Racing v0 (R1 rules, R2 baselines, wasm) | **Live** (#57, #58, #60, #63) |
| M4 Python bridge (`coastal-arena`) | **Live** (#66) |
| M5 two-game viewer | **Slice done** — Nyborg sprites (#67), racing in wasm and viewer (#71, #72), Race these links (#76) |
| Nyborg design / library | **Approved** (#44 rev 3, #49) |
| Customizer (real) | **First slice live** (#73, #76); nav link and polish next |
| Evolution nightly | **Live**; gens 0–399 folded (#68), gens 400–499 in #77; evening slot + quiet-window guard (#69) |
| Champion promotion | **Held** — 99.35% vs Gen 0 at Gen 499 (experimental) |
| License | **PR #62 awaiting Nye** (plus nyborgs#1, site#1, saltmarsh#3) |
| Company website | **Live** at coastal-agentics.github.io |

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

No email gate is open. Still open with Nye (not gated by email): license PR #62 and its assumed decisions; pausing releases; the GitHub App bot identity.

## Open tasks
| Task | Owner | Status |
|---|---|---|
| Check whether the nightly lands nearer its 7:37 PM ET slot (Oct 5 run started 11:17 PM ET) | CoS | next cycle |
| Fold future nightly results with a **merge commit** PR (never squash) | CoS | when there is something new |
| Customizer: link it from the shared nav; polish after Nye plays with it | Blitzwing | next |
| Merge #77 (nightly gens 400–499, merge commit) | CoS | this cycle if green |
| ADR-014 B2–B5 decision (second Rust game now exists) | Shockwave → Nye | due |
| License #62 + nyborgs#1 + site#1 + saltmarsh#3 | Nye | review/merge |
| Bulk `reuse annotate` SPDX headers | CoS | after #62 |
| Saltmarsh MuJoCo world | CoS → Nye | starting it is a gate |

## Known engine / training limits
- Kiter mirrors draw **84.5%** of the time; Kiter beats Charger **69.2%**; triangle draws 8.3%. See `games/tank/BALANCE.md` §0.
- Every evolved champion so far is a Charger above 70% vs Gen 0 (now 99.35% at Gen 499), so none is promotable. Pre-dodge lineage (archived): 89.6–99.9%, all held.
- Swept projectile hits treat targets as stationary within a tick.

## Work budget used
- 2026-09-27 → 2026-10-02: scaffold, rules-v1, ADR-014 B1, GATE-003 M1–M2, dodge + lineage reset (#2–#41).
- 2026-10-03 (Sat): multi-game build-out #43–#66 (engine M1/M2, catalog, racing R1/R2, M4 bridge, Nyborg design/library, field-notes index, license PR #62).
- 2026-10-05 evening cycle: inbox/CI check; #67 confirmed merged; opened and merged #68 (nightly fold-in, merge commit) and #69 (quiet-window guard); STATE refresh; digest.
- 2026-10-06 evening cycle: inbox/CI check (no gate replies); confirmed #71–#76 and nyborgs#2, site#3–#4 merged; opened #77 (nightly fold-in); STATE refresh; digest.
- Limits: 1 scheduled cycle per weekday at 6:16 PM ET + founder-triggered cycles, never weekdays 8 AM–5 PM ET; max 2 concurrent workers.

## Blockers
- No email gates awaiting reply.
- Champion above 70%: do **not** make it the viewer default without a reviewed promotion PR + provenance card.
- #62 is Nye's to merge; nothing else depends on it except site#1 and the SPDX follow-up.

## Next
1. Merge #77; confirm tonight's nightly timing.
2. Blitzwing: Customizer nav link and polish once Nye has played with it.
3. Shockwave: ADR-014 B2–B5 recommendation now that racing exists.
4. Nye: review #62; decide whether 99% under the dodge cap is acceptable or the field needs another change.
