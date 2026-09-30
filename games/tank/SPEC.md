# Tank Arena — SPEC (draft for GATE-002)

Units: engine units (u), ticks at 60 Hz, headings in BAU (65536 = 1 turn, 0 = +X, CCW, Y-up). Numbers are starting values.

## Arena
- 800 × 600, walled (`MatchConfig::duel()` layout): two pillars `Rect` (250,200)–(300,400) and (500,200)–(550,400). Pillars give cover and break sniper sight lines; the open middle lane rewards aggression.
- Same arena for every mode. Fixed, mirrored spawns (via `TankSpawn.pos/heading`, not random):
  - **Duel (1v1):** (100,300) facing 0°; (700,300) facing 180°.
  - **2v2 / FFA-4:** corners (100,100), (700,100), (100,500), (700,500), facing the centre. 2v2 teams are left vs right; FFA gives each tank its own team.

## Tanks
One tank type with three stats (integer levels 1–5). Level 3 equals `TankParams::default()`; scripted matches use 3/3/3 unless a config sets a loadout.

| Stat | Maps to (`TankParams`) | L1 | L2 | **L3 (default)** | L4 | L5 |
|---|---|---|---|---|---|---|
| Attack | projectile_damage | 12 | 16 | **20** | 24 | 28 |
| Speed | max_speed (u/s) / turn_rate (BAU/tick) | 90 / 273 | 105 / 318 | **120 / 364** | 135 / 410 | 150 / 455 |
| Defense | max_hp | 60 | 80 | **100** | 120 | 140 |

Attack changes damage only (cooldown stays fixed, so fire rhythm stays readable). Speed scales hull move and turn together. Defense is plain HP, not damage reduction: simple integer maths, readable hits-to-kill (3–7 at default attack).

**Stat budget (mandatory):** every tank has exactly 9 points across Attack/Speed/Defense, each 1–5; the scripted default is 3/3/3. That gives 19 valid loadouts (integer triples 1–5 summing to 9). Presets (A/S/D): **Balanced** 3/3/3, **Glass Cannon** 5/3/1, **Brawler** 4/1/4, **Scout** 2/5/2.

Hits-to-kill = ⌈defender HP / attacker damage⌉:
| Attack \ Defense | 1 (60) | 2 (80) | 3 (100) | 4 (120) | 5 (140) |
|---|---|---|---|---|---|
| 1 (12) | 5 | 7 | 9 | 10 | 12 |
| 2 (16) | 4 | 5 | 7 | 8 | 9 |
| 3 (20) | 3 | 4 | **5** | 6 | 7 |
| 4 (24) | 3 | 4 | 5 | 5 | 6 |
| 5 (28) | 3 | 3 | 4 | 5 | 5 |

Fixed for everyone:
| Param | Value | Meaning |
|---|---|---|
| radius | 16 | circle collider |
| turret_turn_rate | 546 BAU/tick | turret ~180°/s, world-relative (hull turning does not drag it) |
| fire_cooldown | 45 ticks | 0.75 s; fastest default kill 180 ticks (3 s) |
| projectile_speed | 360 u/s | 6 u/tick, 3× default tank speed |
| projectile_ttl | 120 ticks | range 720 u |
| projectile_spread | 256 BAU | ±1.4° |

At 400 u a shell flies ~67 ticks while a default tank can move ~133 u: long shots need lead and can be dodged. No friendly fire. Tanks block each other and obstacles; walls clamp.

## Match end
Last team standing wins. Simultaneous wipe = draw. Time limit 7200 ticks (120 s); reaching it is a draw. Target: median match 30–60 s, < 10% draws.

## Observation and Action (tank-only)
Keep the engine's current shapes. **Observation:** `tick`; `me` (pos, vel, heading, turret, hp, max_hp, cooldown, radius); `enemies`/`allies` nearest-first (≤ 4; id, team, pos, rel, dist_sq, vel, heading, turret, hp); `projectiles` nearest-first (≤ 8; pos, rel, dist_sq, vel, owner_team); `walls` distances; `arena_size`; `obstacles`. Full information, no fog. Add `los: bool` per tank. **Action:** `throttle`, `turn`, `turret_turn` in [-1, 1], `fire: bool`.

## Scripted policies (in `games/tank`)
Shared: target = `enemies[0]`. Lead aim point `L = rel + vel · (dist / 6)` (dist via `f32::sqrt`, IEEE-exact). Aim with `turn_toward(turret, L, tol)`; fire when aligned, `cooldown == 0` and `los`. Stall = throttle ≠ 0 but `|vel| < 0.2` for 10 ticks → reverse 20 ticks turning +1. Each policy's numbers live in a params struct (the Phase 3 evolution genome).
1. **Charger** — steer at target (`tol 0.2`), throttle 1 until dist < 60, then 0. Aim tol 0.10. Wins by closing fast and trading.
2. **Kiter** — hold 250–350 u. Hull perpendicular to `rel` (circle strafe, throttle 1), bent 30° outward if dist < 250, inward if > 350. Flip strafe direction when a wall < 60, on stall, or every 180 ticks. Aim tol 0.05.
3. **Sniper** — relocate to the point on its half, ≥ 60 from walls, maximising distance to target with LOS; there throttle 0. Aim tol 0.02. If target dist < 250, drive away perpendicular to `rel` for 60 ticks. No LOS for 120 ticks → move along nearest pillar edge until LOS.
Intended triangle: kiter > charger > sniper > kiter. Acceptance: every pairing 55–80% over 200 mirrored seeds.

## Customize tab (web viewer)
The arena viewer gets two tabs: **Watch** (today's viewer) and **Customize**. Depends on engine ask #5 (per-tank `TankParams`).
- Per tank (Blue, Orange): behavior picker (Charger / Kiter / Sniper) and a loadout triangle.
- Triangle: a barycentric point with Attack, Speed, Defense at the corners; stat = 1 + 6·weight, snapped to the nearest of the 19 valid loadouts (exact ties → lower Attack, then lower Speed). Preset buttons set the point.
- Live readout per tank, from the tables above: damage, max speed (u/s), HP, hits-to-kill vs the opponent's HP. E.g. Glass Cannon vs Brawler: 28 dmg kills 120 HP in 5; 24 dmg kills 60 HP in 3.
- **Watch this match** switches to Watch and plays it. Seed, behaviors and loadouts live in the URL query (e.g. `?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`); opening the link replays the match exactly.

## Training indicator
Each tank card shows how its behavior was made. Today: **Scripted** (the three policies above are hand-written). Once the Phase 3 evolution loop exists: generation number, win rate vs the other policies, and a small fitness-over-generations sparkline, all tied to the viewer's generation slider.

## Deferred: loadout kit
Pick N items (e.g. heavy shell, speed boost, armor, smoke that blocks LOS). Needs its own spec and engine work; not in this round.

## Acceptance (customization)
- 3/3/3 vs 3/3/3 replays bit-identical to the default match (same seed → same state hashes).
- Every loadout change alters the readout and the sim (same seed → different replay).
- URL round-trip (encode → load) reproduces the match exactly; the triangle can't produce an invalid loadout (every point snaps to one of the 19).
- Balance: the policy triangle target above is still judged at 3/3/3. Target to check once implemented: no single loadout wins > 70% averaged across all 19 opponent loadouts (per policy, 200 mirrored seeds per pairing).

## What makes it fun to watch
Visible shells you can see coming, and near-miss dodges. Three readable personalities: the rusher, the dancer, the camper. Pillars create peek-and-hide moments. Lead aim makes long hits feel earned. HP bars and a 2-minute cap keep up the drama; comebacks possible (5 hits).

## Known engine limits
- Swept hits (PR #6) treat target tanks as stationary within a tick; negligible at these speeds.
- Movement is simultaneous (PR #6); a tank that would collide stays put that tick. Still evaluate with side-swapped seeds.
- Placeholder bots are lopsided (Wanderer wins ~98% vs Chaser); balance is judged only on the three policies above.

## Engine asks
1. Swept projectile hits (segment vs circle/rect, dot products, no sqrt), so faster shells can't tunnel.
2. `Arena::segment_clear(a, b)` line-of-sight helper, and `los` on `TankObs`.
3. Movement order fairness: alternate id order by tick parity (or resolve moves simultaneously).
4. Optional stationary accuracy: spread 128 BAU when still, 256 when moving (gives the sniper an identity).
5. Per-tank stats: optional `TankSpawn.params: Option<TankParams>` (falls back to `MatchConfig.params`); sim uses each tank's speed, turn rate, HP and damage (radius stays shared). Add `max_hp` to `TankObs`. Bump `REPLAY_FORMAT`.
