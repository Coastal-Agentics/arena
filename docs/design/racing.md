# Racing — game spec (draft)

**Status:** Approved by Nye (racing v0, 2026-10-03 11:50 ET; duplicate Nyborgs may race each other). R1, the rules crate `games/racing`, is built; see "R1 as built" at the end. Numbers marked as starting values get tuned in a racing `BALANCE.md`, the same way the tank numbers were.
**Author:** Blitzwing (Tank Designer-Developer). Part of [game-system.md](game-system.md) §3 (Shockwave), which defines `Rules`, `reward`, `Flat` and replay format 5.
**In one line:** 2–4 cars race 3 laps around a walled track. There are no weapons. The first car home wins. Cars learn by evolution first (like the tanks), and later through PettingZoo.

**Direction (naming and direction only; no code is renamed and nothing in scope changes):** Nye calls the agents **Nyborgs**. A car and a tank are two arena bodies for the same Nyborg, so one customized Nyborg can race and fight. Tracks are data, and Ring is just the first one. Later the engine moves toward customizing both the Nyborgs and the maps and tracks they play on.

Units are the engine's, the same as Tank Arena: engine units (u), ticks at 60 Hz, headings in BAU (65536 = one turn, 0 = +X, counter-clockwise, Y-up). The rules are a `Rules` + `Flat` impl (#30, game-system.md §1) in a new crate, `games/racing`, with game id `racing`. This needs no change to the tank code (see [tank-refit.md](tank-refit.md)).

## Track
- **Shape:** a closed centreline (a list of points) and one width. The walls are the centreline offset by half the width on each side, with mitred corners, so they are straight segments. The infield is solid.
- **Checkpoint gates:** one gate at each centreline point, running from the inner wall corner to the outer wall corner. Gate 0 is the start/finish line. A car must cross the gates **in order and forwards**. Crossing any other gate, or crossing one backwards, counts for nothing.
- **Laps:** a car's first forward crossing of gate 0 starts its race. After that, each crossing of gate 0, once it has passed every other gate, completes a lap. A race is 3 laps (config `laps`).
- **Start grid:** fixed slots in pairs behind the line, all facing along the track.

**First track, "Ring"** (drawn on the tank arena's 800 × 600 canvas):

| Item | Value |
|---|---|
| Centreline, counter-clockwise | (400,100) → (550,100) → (700,250) → (700,350) → (550,500) → (250,500) → (100,350) → (100,250) → (250,100) → back to start |
| Width | 120 u, so the outer walls are at y 40 and 560 and x 40 and 760, and the infield edges are at y 160 and 440 and x 160 and 640 |
| Gates | 9, one at each point; gate 0 is at (400,100) |
| Lap length (centreline) | 1,648.5 u: 800 u of straights and four 45° diagonals of 212.1 u each |
| Grid (4 slots, heading 0) | (370,120), (370,80), (340,120), (340,80); slot 1 is on the inside of the first corner |
| Tick cap | 3,600 ticks (60 s) |

## Cars
One car body. It is a circle collider of radius 12 (the tank's is 16), small enough for four cars to fit across the track. The car drives like the tank hull, with throttle and steering, plus momentum and grip. Every step is plain f32 maths with headings from the engine's table (`angle::dir`), so there is no `sin`, `cos` or `atan2`, and the result is identical natively and in wasm.

Each tick, with `u` = speed along the heading and `w` = sideways speed:
1. **Steer:** heading += steer × turn rate × k(u). k is 0 when standing still and rises to 1 at 60 u/s, so a car can't spin in place. It then falls to 0.5 at top speed, so fast corners need braking or a wide line.
2. **Throttle:** throttle > 0 accelerates up to top speed. Throttle < 0 brakes while moving forward and reverses (up to 60 u/s) once stopped. Throttle 0 coasts down at 60 u/s².
3. **Grip:** the sideways speed `w` loses a fixed share (grip) each tick. High grip feels like rails; low grip lets the car slide, which is the whole drift model.
4. **Move, then collide:**
   - **Walls:** a car overlapping a wall is pushed out along the wall's normal. It loses its speed into the wall and 40% of its speed along it, so scraping costs time.
   - **Cars:** overlapping cars are pushed apart halfway each and swap half their closing speed. All pairs are computed from the same positions and applied together, so car order doesn't matter.
5. **Gates and laps:** each car's path for the tick (old position to new position) is tested against its next gate only.

Validation rejects any config where top speed per tick (top speed ÷ 60) is not below the radius, so nothing can tunnel through a wall: 4.5 u/tick at the fastest setup, against a radius of 12.

**Car setup (the racing "loadout"):** three stats, each 1–5, with exactly 9 points in total. This is the same budget and the same 19 valid setups as the tanks. Starting values:

| Stat | Maps to | L1 | L2 | **L3** | L4 | L5 |
|---|---|---|---|---|---|---|
| Power | acceleration (u/s²) | 180 | 210 | **240** | 280 | 320 |
| Top speed | max speed (u/s) | 200 | 220 | **240** | 255 | 270 |
| Grip | sideways speed removed per tick | 0.15 | 0.20 | **0.25** | 0.30 | 0.35 |

The turn rate is 364 BAU/tick for every car, the tank's Speed 3 value (about 120°/s). The rules make no random draws during a race. The match RNG is used once, in `init`, to shuffle the grid (Fisher–Yates with one `next_u32` per car) unless the config fixes the grid. The seed also feeds the scripted policies' own jitter, just as it does for the tanks.

## Observation and action
**Rich Rust observation** (for scripted policies): own position, velocity, heading, speed, next gate index, laps, progress and place, the other cars nearest first, and the ray distances below. Scripted policies get the track when they are built, from the config.

**Flat encoding (`Flat::encode_obs`), `OBS_LEN = 43`.** It is written into the caller's f32 slice. "Car frame" means (forward, left) relative to the car's heading. After scaling, every value is clamped to [−1, 1].

| Index | Feature | Scaling |
|---|---|---|
| 0–1 | speed forward, sideways | ÷ the car's top speed |
| 2–3 | heading cos, sin | from `angle::dir` |
| 4–5 | position x, y | 2·p/size − 1 (800, 600) |
| 6–14 | wall distance along 9 rays at −90, −60, −35, −15, 0, 15, 35, 60, 90° from the heading | ÷ 300 (1 = no wall within 300 u) |
| 15–16 | next gate's midpoint, car frame | ÷ 300 |
| 17–18 | track direction at the next gate relative to heading, cos and sin | — |
| 19–20 | the gate after next, car frame | ÷ 300 |
| 21 | laps done | ÷ laps |
| 22 | race progress | ÷ (laps × gates) |
| 23 | current place | (place − 1) ÷ (cars − 1), 0 when racing alone |
| 24–41 | 3 opponent slots × 6: present, position (car frame), relative velocity (car frame), ahead in the race | position ÷ 300, velocity ÷ (2 × top speed), flags 0 or 1 |
| 42 | tick | ÷ tick cap |

- **Opponent slots:** nearest first, with ties going to the lower car index. Empty slots are all zeros.
- **Action layout:** `ACTION_LEN = 2`, which is `[throttle, steer]` in [−1, 1]. `decode_action` reads it, and `sanitize` clamps it (NaN becomes 0). Positive steer turns counter-clockwise, as with the tank's `turn`.
- **No separate brake:** negative throttle already brakes. A third control would add an action dimension for RL and a gene for evolution and give the car nothing new.

## How a race ends
- **Finished** (the new `EndReason`; *terminated*): the race ends when every active car has finished (no car is left racing), or 600 ticks (10 s) after the winner crosses the line, whichever comes first. `Outcome.winner` is the winner's team; each car is its own team.
- **TickLimit** (*truncated*): the 3,600-tick (60 s) cap is reached. This applies even when the cap falls inside the 10 s window after a winner, and then the reason is still `TickLimit`. `winner` is the first finisher's team if a car has finished, otherwise `None`.
- **Placings** live in the race state, not in `Outcome`. Finishers are ranked by finish tick, and everyone else by progress at the end. Equal values share a place. A car that has finished becomes inactive (`is_active` = false), which ends its episode (*terminated*) while the others keep racing.
- **Progress** = gates passed + a fraction of the way to the next gate (the car's position projected onto that centreline segment, clamped to 0–1). It depends only on the current state.

## Scoring
- **Evolution fitness (race scoring):** each race scores (cars − place) ÷ (cars − 1), so 1 for a win and 0 for last. The fitness is the mean over all races. Ties are broken by mean finish time (lower is better), then by mean progress. Training races are 4-car, with the candidate against the field, and every grid slot is used, much as tanks play both sides.
- **Done-test, the same bar as GATE-003:** Gen N places ahead of the Gen 0 car in at least 65% of races on 1,000 held-out seeds.
- **Per-tick reward for RL:** `Rules::reward` = (progress now − progress last tick) ÷ gates per lap, so a lap is worth +1. On finishing, a car also gets a bonus of (cars − place) ÷ (cars − 1). Because progress is a state function, the rewards add up to the final progress plus the bonus. Driving back and forth across a gate can't farm reward. The state keeps last tick's progress, so the reward needs no events. Like every reward, it is never recorded in the replay.

## Scripted baselines (Gen 0)
| Policy | Drives | Main params (= genes) |
|---|---|---|
| **Follower** | Steers at a point on the centreline a set distance ahead. Brakes to a corner speed before each corner. | look-ahead, corner speed, braking distance |
| **Cutter** | Racing line: wide on entry, the inside gate end at the apex, wide on exit. Brakes later. | apex margin, entry speed, exit point |
| **Blocker** | Drives the Follower's line, but when a car is close behind and gaining, it moves across to cover that car's line. | block distance, block strength, plus the Follower's params |

The intended character: the Cutter is fastest alone, the Blocker steals places in traffic, and the Follower is the safe middle. Evolution tunes these params and the car setup (Power/Top speed/Grip), as it does for tanks. Later, PettingZoo agents learn from the flat encoding and the reward above.

## Determinism and replays
- **Same result every time:** the same seed, config and per-tick actions give the same final hash. `hash_state` feeds, per car: position, velocity, heading, gates passed, laps, finish tick and last tick's progress.
- **Replay format:** racing replays are **format 5**. This is the shared envelope plus `game: "racing"` and a racing `rules_version` (1 for this spec). The config holds the track, the cars and their setups, `laps` and the tick cap, so `setup_hash` covers the track too.
- **Parity:** racing fixtures join the native-vs-wasm parity set.

## Acceptance
- Each baseline finishes all 3 laps alone on 1,000 seeds, and at least 95% finish in 4-car races.
- Median 4-car race of 20–40 s (Ring: 3 laps ≈ 4,950 u). Under 5% of races hit the tick cap.
- The Cutter's solo lap is faster than the Follower's. In mixed 4-car races over 200 seeds with grid rotation, no baseline wins more than 70%, and no grid slot wins more than 60% of mirror races.
- **No exploits, as tests:**
  - reversing over the line doesn't count a lap;
  - gates crossed out of order don't count;
  - the sum of rewards equals final progress ÷ gates plus the bonus;
  - no car ends a tick overlapping a wall or another car (checked over 1,000 seeds);
  - a config whose speed per tick is not below the radius is rejected.
- **Determinism:** two runs give byte-identical replays, native and wasm hashes agree, and format 5 replays round-trip.

## Milestones (game side)
These are steps inside game-system.md's M3 (racing v0) and M5 (viewer).

| | What | Accepted when |
|---|---|---|
| **R1** (in M3) | `games/racing`: track, car physics, gates, the Ring track, format 5 replays, `Finished` | The determinism and exploit tests above pass, and tank format 4 files still load and verify. |
| **R2** (in M3) | Three baselines, `Flat`, reward, and a racing BALANCE.md | The baseline acceptance targets above. |
| **R3** (in M5) | Watch and Customize in the viewer ([viewer-multi-game.md](viewer-multi-game.md)) | A racing link replays exactly in the browser, and parity CI has racing fixtures. |
| **R4** (after M3) | Evolution for racing (the GATE-003 GA over the baselines' params and the car setup) | A champion beats Gen 0 by at least 65% on 1,000 held-out seeds, and two runs are byte-identical. |

**Needed from the engine (Shockwave):** the `Finished` end reason, the format 5 envelope, and `reward` and `Flat` (M1). Segment walls, rays and circle-vs-segment tests go in `games/racing` first (Shockwave, game-system.md §3). They move to `engine::arena` only if another game needs them. Each car is its own team, so `Outcome.winner: Option<u8>` works unchanged for up to 4 cars.

## R1 as built (games/racing, rules_version 1)
Where the spec left a choice open, R1 settles it as follows. The crate docs (`games/racing/src/rules.rs`) give the exact order inside one step.
- **Identity:** game id `racing`, `RULES_VERSION = 1`, stat keys `power`, `top_speed`, `grip` (levels 1–5, minimum 1, budget 9, 19 setups). The config stores each car's resulting numbers (`CarParams`), not its levels, so retuning the level tables never changes an old replay.
- **Cars per race:** 1–4. A solo race is valid (the acceptance runs baselines alone), and duplicate setups and drivers are allowed.
- **Braking** is 480 u/s² (twice L3 Power), a starting value for BALANCE.md. Steering scales with |forward speed|, so a reversing car steers too, and positive steer is always counter-clockwise. The speed vector is capped at the car's top speed every tick.
- **Collisions:** up to 64 passes per tick, stopping as soon as nothing overlaps. Each pass resolves car pairs first: all pairs are computed from the same positions and applied together, so car order doesn't matter. Each car moves half the overlap. A car already pushed off a wall this tick doesn't move back into it; the other car takes that part. Walls come second in each pass. Overlapping cars are separated to 2 × radius + 0.01 u. Over 1,000 test seeds, no car ends a tick more than 0.001 u inside a wall or another car.
- **Finished cars** are inactive and become ghosts. They drive with the default action (coast), still hit walls, but no longer touch other cars. Without this, a winner coasting to a stop past the line would block the field. They also drop out of the other cars' opponent slots.
- **Progress** is 0 before a car crosses the start line. It is `laps × gates` once the car has finished, and otherwise gates passed + fraction. So the rewards sum to exactly final progress ÷ gates + bonus.
- **Ties:** on a shared first place, `Outcome.winner` is the lowest car index. A solo finish earns a bonus of 1.
- **`Finished`:** a finished race records `EndReason::Finished` (M3a, #56). Racing code can also read `racing::RaceEnd` (`Finished` or `TickLimit`) through `RacingRules::race_end`.
- **Replays:** racing writes format 5 with `game: "racing"` and `rules_version: 1` (`RacingRules::GAME` and `RULES_VERSION`, the same constants as the catalog). Loading checks both, refuses files older than format 5, and rejects any config that fails validation, for example a top speed per tick that is not below the radius.


## R2 as built (baselines)
- **Drivers** live in `games/racing/src/drivers/` (`Behavior::ALL` = the catalog's `follower`, `cutter`, `blocker`). Each is a `Pilot`: pure pursuit of a point ahead on a `Line`, plus a corner speed limit from the planning brake (85% of 480 u/s²). The seed feeds only each driver's own jitter stream (look-ahead ±15%, corner speed ±12%, ±8% per corner, a 20% lift on 10% of corners), never the match RNG.
- **Cutter in traffic:** it lifts behind a car within 40 u ahead in its path or alongside, and only pulls out to pass with 130 u of straight ahead. Without this it won 93% of mixed races.
- **Blocker:** the block is 70 u at strength 0.8, corner speed 164 u/s. It is the weakest baseline in traffic (blocking costs it more than it gains), not the strongest as "intended character" says. This is left to R4's evolution; it isn't an acceptance target.
- **Numbers:** `games/racing/BALANCE.md`, generated by `cargo run -p racing --release --example balance -- 200`. All the baseline acceptance targets above pass.
