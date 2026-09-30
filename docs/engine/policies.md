# Observations, actions and policies

Source: `engine/src/policy.rs`, `engine/src/bots.rs`, `Match::observe` in `engine/src/sim.rs`.

This interface is tank-specific and currently lives in `engine/` (the module doc says "Tank-only
observation/action interface"). ADR-009 (corrected 2026-09-30) records that tank specifics
live in `engine/` today and that moving them to `games/tank` is future work.

## Action

What a tank controller outputs each tick:

```rust
pub struct Action {
    pub throttle: f32,    // forward (+) / reverse (-), fraction of max_speed
    pub turn: f32,        // hull: + counter-clockwise, - clockwise, fraction of turn_rate
    pub turret_turn: f32, // turret, world frame, same sign convention
    pub fire: bool,       // fire if the cooldown allows
}
```

`Action::default()` is all zeros / `false` (do nothing). The sim applies `Action::clamped()`:
analog fields clamped to `[-1, 1]` and NaN becomes 0. Reverse drives at the same maximum speed
as forward. `fire` while the gun is cooling down is ignored, not queued.

## Observation

`Match::observe(id)` builds what tank `id` sees. Policies get **full information**: there's no
fog of war or view cone, and tanks without line of sight are still listed. Each listed tank
carries a `los` flag, so a policy can choose to act on it; the sim itself never checks it.

| Field | Type | Contents |
| --- | --- | --- |
| `tick` | `u32` | `Match::tick()` when observed |
| `me` | `SelfObs` | own `id`, `team`, `pos`, `vel`, `heading`, `turret`, `hp`, `max_hp` (its own, see below), `cooldown`, `radius` (the shared one) |
| `enemies` | `Vec<TankObs>` | living tanks of other teams, nearest first (ties by id), at most `MAX_OBSERVED_TANKS` = 4 |
| `allies` | `Vec<TankObs>` | living teammates except self, same order and cap |
| `projectiles` | `Vec<ProjectileObs>` | every projectile in flight, **including your own team's**, nearest first, at most `MAX_OBSERVED_PROJECTILES` = 8 |
| `walls` | `WallObs` | `left = pos.x`, `right = size.x - pos.x`, `bottom = pos.y`, `top = size.y - pos.y` |
| `arena_size` | `Vec2` | arena size |
| `obstacles` | `Vec<Rect>` | every obstacle |

`TankObs` = `id`, `team`, `pos`, `rel` (= `pos - me.pos`), `dist_sq` (= `rel.length_squared()`),
`vel`, `heading`, `turret`, `hp`, `max_hp`, `los`. `ProjectileObs` = `pos`, `rel`, `dist_sq`,
`vel`, `owner_team`. Velocities are in units per tick. Headings are BAU ([world](world.md#headings)).

- **`max_hp`** (in `TankObs` and `SelfObs`) is that tank's own `TankParams::max_hp`
  (`Match::tank_params(id)`), so it differs between tanks when spawns carry
  [per-tank params](world.md#per-tank-params). `hp / max_hp` is the fraction left.
- **`los`** is `arena.segment_clear(me.pos, other.pos)`, between the two tank **centres**
  ([line of sight](world.md#line-of-sight)): walls and obstacles block it, grazing an obstacle
  corner or edge counts as blocked, and **other tanks never block it**. It's computed for
  allies too. It is a sight line, not a shot prediction: shells start `radius + 1` ahead of
  the centre along the turret (plus spread), so a shot can be blocked when `los` is true or
  get through when it's false, and an enemy tank on the line takes the hit first (shells
  pass through allies).

Test `observation_reports_max_hp_and_los` covers per-tank `max_hp`, a tank hidden behind an
obstacle and a tank standing on the sight line.

`step_policies` builds every tank's observation from the same tick-start state before any
action is applied.

## The `Policy` trait

```rust
pub trait Policy {
    fn act(&mut self, obs: &Observation) -> Action;
}
impl<F: FnMut(&Observation) -> Action> Policy for F { … }
```

A policy must be deterministic given its own state and the observations; if it needs
randomness it should own a seeded RNG ([determinism](determinism.md#policies-bring-their-own-randomness)).
Any closure `FnMut(&Observation) -> Action` is a policy:

```rust
use engine::{Action, Match, MatchConfig, Observation};

let mut sit = |_: &Observation| Action::default();
let mut spin = |_: &Observation| Action { turn: 1.0, ..Default::default() };
let mut m = Match::new(MatchConfig::duel(), 1);
let outcome = m.run(&mut [&mut sit, &mut spin]); // draw at the 7200-tick limit
```

(This is a doc test on `Policy`, so `cargo test` runs it.)

`Match::step_policies` and `Match::run` take `&mut [&mut dyn Policy]`, indexed by tank id.
The policy for a dead tank isn't called.

## Built-in policies

Both live in `engine::bots` and are placeholders "for the CLI smoke run, tests, and the
viewer" (module doc). They use `angle::turn_toward(h, target, tol)`. It returns `0` when the
target is in front and the sine of the angle to it is within `tol`. Otherwise it returns `1`
(turn counter-clockwise) or `-1` (clockwise); a target directly behind gives `1`.

### `Chaser` (stateless)

Against the nearest living enemy (`enemies[0]`):

- `throttle` = 1.0 while `dist_sq > 220²`, else 0.0;
- `turn` = `turn_toward(heading, rel, 0.2)`;
- `turret_turn` = `turn_toward(turret, rel, 0.08)`;
- `fire` = the turret result is 0 (aligned). It doesn't check line of sight, so it can fire
  into obstacles.

No enemy: `Action::default()`.

### `Wanderer::new(seed)`

Owns a `ChaCha8Rng::seed_from_u64(seed)`.

- `throttle` is always 0.8;
- `turn` is -1, 0 or +1 (`next_u32() % 3 - 1`), held for 20 to 59 ticks
  (`20 + next_u32() % 40`), then redrawn;
- the turret and `fire` work like Chaser's (tolerance 0.08); no enemy → `turret_turn = 0`,
  `fire = false`. It still drives and turns.

Observed, not designed: in `engine-cli`'s duel (Chaser team 0, Wanderer team 1), seeds 0–199
end 195 Wanderer wins to 5 Chaser wins, all by `last_standing` (run on 2026-09-30).

## games/tank

`games/tank` currently exports only `GAME_NAME = "Tank Arena"` and `tick_hz()` (which returns
`engine::TICK_HZ`). It has no rules, observations, actions or policies yet. The Tank Arena spec
(`games/tank/SPEC.md`, GATE-002) is merged; its scripted policies (Charger, Kiter, Sniper) and
the stat-to-params mapping for loadouts belong in `games/tank`, not in the engine. The engine
side of the spec's asks is described in [world](world.md#per-tank-params) and above.
