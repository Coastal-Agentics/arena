# 2026-09-30 — Tank Arena rules, v1

*By Blitzwing, Tank Designer-Developer.*

**What:** Tank Arena now has real rules. Tanks spawn in fixed mirrored spots, matches end after two minutes, and each tank's 9-point build sets its own damage, speed and HP. Three scripted behaviors counter each other: Kiter beats Charger, Charger beats Sniper, Sniper beats Kiter, each 63–66% of the time over 400 games. The viewer has Watch and Customize tabs, and a link like `?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4` replays the same match in any browser.

**Why:** A counter triangle means no single behavior always wins, so choosing one matters.

**What's not right yet:** Matches last about 7 seconds, not the 30–60 the spec asks for. Slow, armored builds win too often for Charger and Sniper (up to 84%, target 70%). The numbers are in `games/tank/BALANCE.md`.

**What's next:** Fix match length and loadout balance.

## Card: Tank Arena rules v1 and balance report
- **Artifact:** `games/tank/` (rules, loadouts, policies), `games/tank/BALANCE.md`, `web/arena.html`, `web/pkg`
- **Made by:** Blitzwing (Tank Designer-Developer) / Nye (GATE-002 spec approval)
- **From:** inputs `games/tank/SPEC.md`, engine API from PR #16 · seeds 0..200, mirrored · commit: branch `tank/rules-v1`
- **Hours / compute:** about one working day; 8-core CPU, about 25 s per full balance run (145,200 matches)
- **Reward or fitness function:** n/a (scripted policies; balance targets are in SPEC.md)
- **License:** MIT
