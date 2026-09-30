# The world

Source: `engine/src/arena.rs`, `engine/src/angle.rs`, `engine/src/sim.rs`.

## Arena and coordinates

An `Arena` is a rectangle from `(0, 0)` to `size` (`Vec2`, from `glam`), plus a list of solid
axis-aligned obstacles (`Rect { min, max }`). The frame is **Y-up**: `(0, 0)` is the
bottom-left corner, `+Y` is up. Units are plain world units; with the default params a tank
is 16 units in radius and drives 120 units per second.

- The walls are the arena edges. A tank's centre is clamped to `[r, size - r]` on each axis.
- Obstacles block tanks (circle vs rectangle overlap; touching is not overlap) and stop
  projectiles (swept segment test against the closed rectangle, so touching an edge counts).
- A projectile is stopped by a wall when its path goes strictly outside `[0, size]`.
- There is nothing else in the world: no terrain, pickups or line-of-sight rules.

## Headings

Headings are `Heading = u16` binary angle units (BAU): 65536 per full turn, `0` = `+X`,
`16384` (`QUARTER_TURN`) = `+Y`, increasing counter-clockwise. Adding wraps, so turning never
needs normalising. `angle::from_degrees(90) == QUARTER_TURN`.

`angle::sin`, `cos` and `dir` read a 1024-entry table built at compile time from a Taylor
series (resolution 64 BAU, about 0.35°). The sim never calls platform trig. See
[determinism](determinism.md).

## Tanks

Tanks are circles. `Tank` holds:

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `usize` | Index in `Match::tanks()` and in each tick's action list |
| `team` | `u8` | Allies share a team; projectiles pass through their own team |
| `pos` | `Vec2` | Centre |
| `vel` | `Vec2` | Movement actually applied last tick, **units per tick** (blocked axes are zero) |
| `heading` | `Heading` | Hull direction; the tank drives along it |
| `turret` | `Heading` | Turret direction in the world frame (it does not follow the hull) |
| `hp` | `i32` | Hit points; not clamped, so it can go negative on the killing hit |
| `cooldown` | `u32` | Ticks until the gun can fire (0 = ready) |
| `alive` | `bool` | Set to `false` when `hp <= 0`. Dead tanks stay in the list but don't move, fire, block movement or get hit |

A tank's turret starts aligned with its hull.

## Tank parameters

`TankParams` is shared by every tank in a match. `TankParams::default()`:

| Field | Default | Unit | Notes |
| --- | --- | --- | --- |
| `radius` | 16.0 | units | |
| `max_speed` | 120.0 | units/s | 2 units per tick at throttle 1 |
| `turn_rate` | 364 | BAU/tick | ≈2°/tick, ≈120°/s |
| `turret_turn_rate` | 546 | BAU/tick | ≈3°/tick, ≈180°/s |
| `max_hp` | 100 | HP | |
| `fire_cooldown` | 45 | ticks | a tank holding `fire` shoots on ticks 1, 46, 91, … |
| `projectile_speed` | 360.0 | units/s | 6 units per tick |
| `projectile_ttl` | 120 | ticks | at most 120 moves, i.e. 720 units from the muzzle |
| `projectile_damage` | 20 | HP | five hits destroy a default tank |
| `projectile_spread` | 256 | BAU | each shot deviates by a uniform integer in `[-256, 256]` BAU (≈±1.4°) |

The 1, 46, 91 firing ticks and the 720-unit reach were checked by running the engine, not only
read off the code.

## Projectiles

`Projectile { owner, team, pos, vel, ttl, damage }`. A shot spawns `radius + 1` units in front
of the tank's centre along the turret heading (plus spread), moves in a straight line at
`vel` units per tick, and is removed when it:

- hits a living tank of another team (it damages only the first one it reaches),
- touches a wall or obstacle, or
- runs out of `ttl`.

Projectiles don't collide with each other or with tanks on their own team. A new shot does
not move on the tick it is fired; its first move is the next tick.

## Match config

`MatchConfig { arena, tanks: Vec<TankSpawn>, params: TankParams, max_ticks }`. With a seed,
it is everything needed to reproduce a match.

`TankSpawn { team, pos: Option<Vec2>, heading: Option<Heading> }`. `None` means "draw it from
the match RNG" (see [determinism](determinism.md#what-draws-from-the-match-rng)). An explicit
`pos` is used as given; it is not checked against walls, obstacles or other tanks.

`MatchConfig::duel()` is the only built-in config and the one `engine-cli` and the viewer use:

- an 800 × 600 arena,
- two obstacles, `(250, 200)–(300, 400)` and `(500, 200)–(550, 400)`,
- two tanks, team 0 and team 1, both with random position and heading,
- `TankParams::default()`,
- `max_ticks = 120 * TICK_HZ` = 7200 ticks (2 minutes of sim time).

## Events

`Match::events()` lists what happened during the most recent step (it is cleared at the
start of each step):

- `Event::Fired { tank }`, in tank id order;
- `Event::Hit { target, owner, damage }`, in projectile list order;
- `Event::Destroyed { tank }`, in tank id order.

Events are not stored in replays.

## How a match ends

After every step (and once in `Match::new`) the sim checks, in this order:

1. No living tanks: draw, `EndReason::AllDestroyed`. This covers a mutual kill on the same
   tick, and a config with no tanks (over at tick 0).
2. Exactly one team has living tanks, and at least one tank of another team exists:
   that team wins, `EndReason::LastStanding`.
3. `tick >= max_ticks`: draw, `EndReason::TickLimit`.

So a win on the last tick beats the tick limit. A match with only one team can only end by
the tick limit. The result is an `Outcome { winner: Option<u8>, ticks, reason }`. Once it is
set, further `step` calls change nothing and return it.
