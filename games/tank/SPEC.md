# Tank Arena — SPEC (GATE-002, approved; amended 2026-09-30; dodge amendment 2026-10-02 pending founder approval)

Units: engine units (u), ticks at 60 Hz, headings in BAU (65536 = 1 turn, 0 = +X, CCW, Y-up). The policy numbers are starting values; the tuned ones are in the policy params structs (`games/tank/src/policies/`), with results in `BALANCE.md`.

## Arena
- 800 × 600, walled (`MatchConfig::duel()` layout): two pillars `Rect` (250,200)–(300,400) and (500,200)–(550,400). Pillars give cover and break sniper sight lines; the open middle lane rewards aggression.
- Same arena for every mode. Fixed, mirrored spawns (via `TankSpawn.pos/heading`, not random):
  - **Duel (1v1):** (100,300) facing 0°; (700,300) facing 180°.
  - **2v2 / FFA-4:** corners (100,100), (700,100), (100,500), (700,500), facing the centre. 2v2 teams are left vs right; FFA gives each tank its own team.

## Tanks
One tank type with three stats (integer levels 1–5). Level 3 equals `TankParams::default()` except HP (650 instead of 100); the Tank Arena match config uses the 3/3/3 params as its shared params, and scripted matches use 3/3/3 unless a config sets a loadout.

*Retune (2026-09-30, approved as a GATE-002 amendment):* HP ×6.5 and geometric tables. The first tables gave 7-second matches, and slow armoured builds won 84% of the time because Speed did nothing for a tank that stands and shoots. Evidence: `games/tank/BALANCE.md`.

| Stat | Maps to (`TankParams`) | L1 | L2 | **L3 (default)** | L4 | L5 |
|---|---|---|---|---|---|---|
| Attack | projectile_damage | 14 | 17 | **20** | 24 | 29 |
| Speed | max_speed (u/s) / turn_rate (BAU/tick) | 90 / 273 | 105 / 318 | **120 / 364** | 135 / 410 | 150 / 455 |
| Speed | fire_cooldown (ticks) | 64 | 54 | **45** | 37 | 31 |
| Defense | max_hp | 460 | 550 | **650** | 790 | 940 |

Each stat is about ×1.2 per level (damage, fire rate, HP), so moving a point between stats keeps damage × fire rate × HP roughly constant: no build wins a straight exchange of fire by construction, and Speed's movement is the tie-breaker. Attack changes damage only. Speed scales hull move, turn and reload together (a faster tank also shoots faster). Defense is plain HP, not damage reduction: simple integer maths; default hits-to-kill is 33.

**Stat budget (mandatory):** every tank has exactly 9 points across Attack/Speed/Defense, each 1–5; the scripted default is 3/3/3. That gives 19 valid loadouts (integer triples 1–5 summing to 9). Presets (A/S/D): **Balanced** 3/3/3, **Glass Cannon** 5/3/1, **Brawler** 4/1/4, **Scout** 2/5/2.

Hits-to-kill = ⌈defender HP / attacker damage⌉:
| Attack \ Defense | 1 (460) | 2 (550) | 3 (650) | 4 (790) | 5 (940) |
|---|---|---|---|---|---|
| 1 (14) | 33 | 40 | 47 | 57 | 68 |
| 2 (17) | 28 | 33 | 39 | 47 | 56 |
| 3 (20) | 23 | 28 | **33** | 40 | 47 |
| 4 (24) | 20 | 23 | 28 | 33 | 40 |
| 5 (29) | 16 | 19 | 23 | 28 | 33 |

Fixed for everyone:
| Param | Value | Meaning |
|---|---|---|
| radius | 16 | circle collider |
| turret_turn_rate | 546 BAU/tick | turret ~180°/s, world-relative (hull turning does not drag it) |
| projectile_speed | 360 u/s | 6 u/tick, 3× default tank speed |
| projectile_ttl | 120 ticks | range 720 u |
| projectile_spread | 256 BAU | ±1.4° |

At 400 u a shell flies ~67 ticks while a default tank can move ~133 u: long shots need lead and can be dodged. No friendly fire. Tanks block each other and obstacles; walls clamp.

## Match end
Last team standing wins. Simultaneous wipe = draw. Time limit 7200 ticks (120 s); reaching it is a draw. Target: median match 30–60 s, < 10% draws.

## Observation and Action (tank-only)
Keep the engine's current shapes. **Observation:** `tick`; `me` (pos, vel, heading, turret, hp, max_hp, cooldown, radius); `enemies`/`allies` nearest-first (≤ 4; id, team, pos, rel, dist_sq, vel, heading, turret, hp, max_hp, los); `projectiles` nearest-first (≤ 8; pos, rel, dist_sq, vel, owner_team); `walls` distances; `arena_size`; `obstacles`. Full information, no fog. `los: bool` per tank is `Arena::segment_clear` between centres. **Action:** `throttle`, `turn`, `turret_turn` in [-1, 1], `fire: bool`.

## Scripted policies (in `games/tank`)
Shared: target = `enemies[0]`. Lead aim point `L = rel + vel · (dist / 6)` (dist via `f32::sqrt`, IEEE-exact). Aim with `turn_toward(turret, L, tol)`; fire when aligned, `cooldown == 0` and `los`. Stall = throttle ≠ 0 but `|vel| < 0.2` for 10 ticks → reverse 20 ticks turning +1. Each policy's numbers live in a params struct (the Phase 3 evolution genome).

**Dodge reflex (all three policies; amendment 2026-10-02, needs founder approval as a GATE-002 amendment).** Each tick, an enemy shell is a *threat* if, assuming we stand still, its closest approach to our centre comes within the **look-ahead** (ticks) and passes closer than `radius + `**threshold** (u). Each threatening shell is rolled once, the first tick it is a threat, against the **strength** (chance of reacting; seeded ChaCha8 stream `seed ^ DODGE_SALT`, so rolls never shift a policy's other timing). The tank drives (forward or reverse, whichever end is nearer) perpendicular to the most urgent shell it reacts to, away from its closest-approach point; that overrides the policy's own steering for the tick (stall recovery still comes first). Shells it chose to ignore stay ignored for their flight. Faster tanks get out of the way sooner, so Speed buys survival as well as reload.

| Policy | Look-ahead (ticks) | Threshold (u) | Strength |
|---|---|---|---|
| Charger | 23 | 5.3 | 0.65 |
| Kiter | 11 | 0.66 | 0.95 |
| Sniper | 15 | 5.4 | 0.61 |

Why not always dodge: at strength 1 a dodging tank is almost untouchable at range (kiter vs sniper and the kiter and sniper mirrors end in draws), and the first attempt (charger and sniper at a 20-tick look-ahead, kiter not dodging) broke the triangle (kiter beat charger 2%, sniper beat kiter up to 100%). The kiter's tiny threshold is deliberate: it reacts only at the last moment, which is what a dancer that is always moving can afford. Before/after numbers: `BALANCE.md` §0.
1. **Charger** — steer at target (`tol 0.2`), throttle 1 until dist < 60, then 0. Aim tol 0.10. Wins by closing fast and trading. *Shipped (2026-10-02):* aim tol 0.044, weaves ±14° every 30 ± 10 ticks until within 150 u, dodges as above.
2. **Kiter** — hold 250–350 u. Hull perpendicular to `rel` (circle strafe, throttle 1), bent 30° outward if dist < 250, inward if > 350. Flip strafe direction when a wall < 60, on stall, or every 180 ticks. Aim tol 0.05. *Shipped (2026-10-02):* band 250–348 u, timed flip every 190 ± 40 ticks, aim tol 0.048, dodges as above.
3. **Sniper** — relocate to the point on its half, ≥ 60 from walls, maximising distance to target with LOS; there throttle 0. Aim tol 0.02. If target dist < 250, drive away perpendicular to `rel` for 60 ticks. No LOS for 120 ticks → move along nearest pillar edge until LOS. *Shipped (2026-10-02):* aim tol 0.025, evades below 238 u, dodges as above.
Intended triangle: kiter > charger > sniper > kiter. Acceptance: every pairing 55–80% over 200 mirrored seeds. With dodging on (2026-10-02): kiter > charger 69.2%, charger > sniper 64.5%, sniper > kiter 75.0%; median match 44.6 s, draws 8.3%; strongest loadouts Charger 5-1-3 58.8%, Kiter 5-3-1 25.0%, Sniper 5-1-3 59.2% (all pass; `BALANCE.md`). Before dodging: 76.2% / 72.5% / 65.5%, median 34.4 s, draws 3.2%, strongest loadouts 65.4% / 42.7% / 61.5%.

## Customize tab (web viewer)
The arena viewer has two tabs: **Watch** (the match) and **Customize**. Builds use per-tank `TankParams` (engine ask #5).
- Per tank (Blue, Orange): behavior picker (Charger / Kiter / Sniper) and a loadout triangle.
- Triangle: a barycentric point with Attack, Speed, Defense at the corners; stat = 1 + 6·weight, snapped to the nearest of the 19 valid loadouts (exact ties → lower Attack, then lower Speed). Preset buttons set the point.
- Live readout per tank, from the tables above: damage, reload, max speed (u/s), HP, hits-to-kill vs the opponent's HP. E.g. Glass Cannon vs Brawler: 29 dmg kills 790 HP in 28; 24 dmg kills 460 HP in 20.
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
Visible shells you can see coming, and near-miss dodges. Three readable personalities: the rusher, the dancer, the camper. Pillars create peek-and-hide moments. Lead aim makes long hits feel earned. HP bars and a 2-minute cap keep up the drama; comebacks possible (33 hits at 3/3/3).

## Known engine limits
- Swept hits (PR #6) treat target tanks as stationary within a tick; negligible at these speeds.
- Movement is simultaneous (PR #6); a tank that would collide stays put that tick. Still evaluate with side-swapped seeds.
- Placeholder bots are lopsided (Wanderer wins ~98% vs Chaser); balance is judged only on the three policies above.

## Engine asks (all delivered: 1 and 3 in #6, 2, 4 and 5 in #16)
1. Swept projectile hits (segment vs circle/rect, dot products, no sqrt), so faster shells can't tunnel.
2. `Arena::segment_clear(a, b)` line-of-sight helper, and `los` on `TankObs`.
3. Movement order fairness: alternate id order by tick parity (or resolve moves simultaneously).
4. Optional stationary accuracy: spread 128 BAU when still, 256 when moving (gives the sniper an identity). Delivered as `projectile_spread_still`; Tank Arena leaves it off (see BALANCE.md).
5. Per-tank stats: optional `TankSpawn.params: Option<TankParams>` (falls back to `MatchConfig.params`); sim uses each tank's speed, turn rate, HP and damage (radius stays shared). Add `max_hp` to `TankObs`. Bump `REPLAY_FORMAT`.
