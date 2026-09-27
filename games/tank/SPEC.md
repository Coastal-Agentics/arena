# Tank Arena — SPEC (draft for GATE-002)

Units: engine units (u), ticks at 60 Hz, headings in BAU (65536 = 1 turn, 0 = +X, CCW, Y-up). Numbers are starting values, tuned later by playtest and self-play.

## Arena
- 800 × 600, walled (`MatchConfig::duel()` layout): two pillars `Rect` (250,200)–(300,400) and (500,200)–(550,400). Pillars give cover and break sniper sight lines; the open middle lane rewards aggression.
- Same arena for every mode. Fixed, mirrored spawns (via `TankSpawn.pos/heading`, not random):
  - **Duel (1v1):** (100,300) facing 0°; (700,300) facing 180°.
  - **2v2 / FFA-4:** corners (100,100), (700,100), (100,500), (700,500), facing the centre. 2v2 teams are left vs right; FFA gives each tank its own team.

## Tanks (symmetric; policies differ only in behaviour)
| Param (`TankParams`) | Value | Meaning |
|---|---|---|
| radius | 16 | circle collider |
| max_speed | 120 u/s | 2 u/tick, forward or reverse |
| turn_rate | 364 BAU/tick | hull ~120°/s |
| turret_turn_rate | 546 BAU/tick | turret ~180°/s, world-relative (hull turning does not drag it) |
| max_hp | 100 | |
| projectile_damage | 20 | 5 hits to kill |
| fire_cooldown | 45 ticks | 0.75 s; fastest kill 180 ticks (3 s) |
| projectile_speed | 360 u/s | 6 u/tick, 3× tank speed |
| projectile_ttl | 120 ticks | range 720 u |
| projectile_spread | 256 BAU | ±1.4° |

At 400 u a shell flies ~67 ticks while a target can move ~133 u: long shots need lead and can be dodged. No friendly fire. Tanks block each other and obstacles; walls clamp.

## Match end
Last team standing wins. Simultaneous wipe = draw. Time limit 7200 ticks (120 s); at the limit the team with more total HP wins, equal HP = draw (see Engine asks). Target: median match 30–60 s, < 10% draws.

## Observation and Action (tank-only)
Keep the engine's current shapes. **Observation:** `tick`; `me` (pos, vel, heading, turret, hp, max_hp, cooldown, radius); `enemies`/`allies` nearest-first (≤ 4; id, team, pos, rel, dist_sq, vel, heading, turret, hp); `projectiles` nearest-first (≤ 8; pos, rel, dist_sq, vel, owner_team); `walls` distances; `arena_size`; `obstacles`. Full information, no fog. Add `los: bool` per tank. **Action:** `throttle`, `turn`, `turret_turn` in [-1, 1], `fire: bool`.

## Scripted policies (in `games/tank`)
Shared: target = `enemies[0]`. Lead aim point `L = rel + vel · (dist / 6)` (dist via `f32::sqrt`, IEEE-exact). Aim with `turn_toward(turret, L, tol)`; fire when aligned, `cooldown == 0` and `los`. Stall = throttle ≠ 0 but `|vel| < 0.2` for 10 ticks → reverse 20 ticks turning +1. Each policy's numbers live in a params struct (the Phase 3 evolution genome).
1. **Charger** — steer at target (`tol 0.2`), throttle 1 until dist < 60, then 0. Aim tol 0.10. Wins by closing fast and trading.
2. **Kiter** — hold 250–350 u. Hull perpendicular to `rel` (circle strafe, throttle 1), bent 30° outward if dist < 250, inward if > 350. Flip strafe direction when a wall < 60, on stall, or every 180 ticks. Aim tol 0.05.
3. **Sniper** — relocate to the point on its half, ≥ 60 from walls, maximising distance to target with LOS; there throttle 0. Aim tol 0.02. If target dist < 250, drive away perpendicular to `rel` for 60 ticks. No LOS for 120 ticks → move along nearest pillar edge until LOS.
Intended triangle: kiter > charger > sniper > kiter. Acceptance: every pairing 55–80% over 200 mirrored seeds.

## What makes it fun to watch
Visible shells you can see coming, and near-miss dodges. Three readable personalities: the rusher, the dancer, the camper. Pillars create peek-and-hide moments. Lead aim makes long hits feel earned. HP bars and a 2-minute cap keep drama; comebacks possible (5 hits).

## Known engine limits
- Hits check only the projectile end point per tick. Safe while speed < 2·radius per tick (~1900 u/s); tunnelling appears beyond that.
- Tanks move in id order: slight bias to tank 0 in collisions. Evaluate with side-swapped seeds.
- Placeholder bots are lopsided (Wanderer wins ~98% vs Chaser); balance is judged only on the three policies above.

## Engine asks
1. Swept projectile hits (segment vs circle/rect, dot products, no sqrt), so faster shells can't tunnel.
2. `Arena::segment_clear(a, b)` line-of-sight helper, and `los` on `TankObs`.
3. HP tiebreak at the tick limit (`EndReason::HpTiebreak`, optional in `MatchConfig`).
4. Movement order fairness: alternate id order by tick parity (or resolve moves simultaneously).
5. Optional stationary accuracy: spread 128 BAU when still, 256 when moving (gives the sniper an identity).
