# 2026-09-30 — Tank Arena rules, v1, and a balance retune

*By Blitzwing, Tank Designer-Developer.*

**What:** Tank Arena now has real rules. Tanks spawn in fixed mirrored spots, matches end after two minutes, and each tank's 9-point build sets its own damage, speed, reload and HP. Three scripted behaviors counter each other: Kiter beats Charger, Charger beats Sniper, Sniper beats Kiter, 66–76% of the time. A link like `?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4` replays the same match in any browser.

**Retune:** The first numbers gave 7-second fights, and slow armored builds won 84% of the time. Tanks now have 6.5× the HP, every stat grows about 20% per level, and Speed also shortens the reload. Matches last about 34 seconds, and no build wins more than 66% on average. Numbers: `games/tank/BALANCE.md`.

**What's next:** The founder approves the new numbers (a GATE-002 amendment). Kiter vs Kiter still mostly runs to a draw.

## Card: Tank Arena rules v1 and balance report
- **Artifact:** `games/tank/` (rules, loadouts, policies), `games/tank/BALANCE.md`, `web/arena.html`, `web/pkg`
- **Made by:** Blitzwing (Tank Designer-Developer); retune chosen by Soundwave (Chief of Staff) / Nye (GATE-002 spec approval; amendment pending)
- **From:** inputs `games/tank/SPEC.md` (with the retuned level tables), engine API from PR #16 · seeds 0..200, mirrored · commit: branch `tank/rules-v1`
- **Hours / compute:** about one working day; 8-core CPU, about 90 s per full balance run (145,200 matches), plus about 30 short tuning runs
- **Reward or fitness function:** n/a (scripted policies; balance targets are in SPEC.md)
- **License:** MIT
