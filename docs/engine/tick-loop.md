# The tick loop

Source: `engine/src/lib.rs` (`TICK_HZ`, `DT`), `engine/src/generic/match_loop.rs`
(`Match::new`, `step`, `step_policies`, `run`) and `engine/src/sim.rs` (`TankRules::step`,
`TankRules::outcome`).

## Fixed 60 Hz step

```rust
pub const TICK_HZ: u32 = 60;
pub const DT: f32 = 1.0 / TICK_HZ as f32;
```

Every call to `Match::step` advances the world by exactly one tick of `DT` seconds. There is no
variable timestep and no wall clock anywhere in the engine: nothing reads the time, so a match
runs the same at any speed. Per-second params (`max_speed`, `projectile_speed`) are multiplied
by `DT` once to get per-tick amounts; turn rates are already per tick.

"Time" in a match is `Match::tick()`, the number of steps taken so far (starts at 0).

## Driving a match

| Call | What it does |
| --- | --- |
| `Match::new(config, seed)` | Seeds the RNG, builds the initial state (`Rules::init`; for tanks: spawns them), checks the end condition once (`Rules::outcome` at tick 0) |
| `Match::step(&[Action])` | One tick with explicit actions. `actions[i]` drives tank `i`; missing entries are `Action::default()`, extra entries are ignored; returns `Some(Outcome)` once over |
| `Match::step_policies(&mut [&mut dyn Policy])` | Builds every observation from the current state, asks each active (for tanks: living) agent's policy for an action (dead tanks and tanks without a policy get `Action::default()`, and their policy isn't called), then calls `step` |
| `Match::run(&mut [&mut dyn Policy])` | Calls `step_policies` until the match ends; returns the `Outcome` |

Once the match is over, `step` does nothing and returns the stored outcome; the tick counter
and history stop growing.

`engine::Match` is `engine::generic::Match<TankRules>`: the calls above are the generic
match loop, and everything tank-specific happens inside the `Rules` functions it calls
([ADR-014](../DECISIONS.md) step B1). The generic `Match::step` does, in order:

1. clear the previous step's events;
2. build one action per agent (`Rules::agents`): `actions[i]` or the default, passed through
   `Rules::sanitize`;
3. `Rules::step(config, state, actions, rng, events)`, the only place besides `Rules::init`
   that gets the match RNG;
4. push the sanitized actions onto the history, `tick += 1`;
5. store `Rules::outcome(config, state, tick)`.

## Inside one step (Tank Arena)

This is the order in `TankRules::sanitize` and `TankRules::step`, which the determinism
guarantees depend on. Every tank
uses its own params (`Match::tank_params(i)`, see [per-tank params](world.md#per-tank-params))
for `turn_rate`, `turret_turn_rate`, `max_speed` and everything about its gun; the collision
radius `r` is the shared `MatchConfig::params.radius`.

0. **Record actions** (`TankRules::sanitize`, called by the generic step). For each tank id,
   take `actions[i]` (or the default) and clamp it: `throttle`, `turn` and `turret_turn` to
   `[-1, 1]`, NaN to 0. This clamped list is what goes into the history (and the replay).
   Actions for dead tanks are recorded but have no effect.
1. **Turn and plan moves** (each living tank, against the tick-start positions of all tanks):
   - `heading += trunc(turn * turn_rate)`, `turret += trunc(turret_turn * turret_turn_rate)`
     (wrapping `u16` arithmetic);
   - desired move = `dir(heading) * throttle * max_speed * DT`;
   - the move is tried one axis at a time, **x then y**. Each axis is clamped to the walls;
     if the result overlaps an obstacle or another living tank's tick-start position, that
     axis doesn't move and its velocity is 0. So a tank slides along a wall or obstacle on the
     free axis.
2. **Apply moves simultaneously.** If a tank's planned position overlaps another living
   tank's planned position, it stays where it was (velocity 0). Repeat until nothing
   changes. This always settles, because a pass can only send tanks back to their
   tick-start positions. The result doesn't depend on tank ids (the test
   `movement_does_not_depend_on_tank_id_order` swaps ids and gets the same positions).
3. **Fire**, in tank id order. Each living tank takes its new position, decrements
   `cooldown` if it is above 0, and if `fire` is set and `cooldown == 0`:
   - picks its effective spread: `projectile_spread_still` if that is set **and** the tank's
     applied velocity this tick is exactly zero, else `projectile_spread`
     ([stationary accuracy](world.md#stationary-accuracy));
   - draws a deviation from the match RNG (only if the effective spread is above 0),
   - spawns a projectile `radius + 1` in front of it along `turret + deviation`, with its own
     `projectile_speed`, `projectile_ttl` and `projectile_damage`,
   - sets `cooldown` to its own `fire_cooldown` and emits `Event::Fired`.
   New shots are held aside; they don't move this tick.
4. **Move projectiles** that already existed, in list order. Each sweeps its whole segment
   `pos → pos + vel`:
   - the earliest entry into a living enemy tank (at its post-move position), compared with
     the earliest wall or obstacle contact; the tank wins ties with the wall, and between
     tanks the lower id wins ties;
   - on a tank hit: subtract `damage` from its `hp`, emit `Event::Hit`, remove the shot;
   - on a wall or obstacle: remove the shot;
   - otherwise remove it if `ttl <= 1`, else `ttl -= 1` and keep it.
5. **Finish the tick.** Append this tick's new shots to the projectile list. Every living
   tank with `hp <= 0` becomes dead (`vel = 0`, `Event::Destroyed`). That ends
   `TankRules::step`; the generic loop then pushes the recorded actions onto the history,
   does `tick += 1`, and checks the end conditions with `TankRules::outcome`
   ([world](world.md#how-a-match-ends)).

Consequences worth knowing:

- Deaths are applied after all projectiles move, so a tank can take several hits in one
  tick, and a tank that dies still moved and fired that tick. Two tanks can kill each other
  on the same tick (`AllDestroyed`).
- A projectile that lands its hit on its last `ttl` tick still counts: the hit test happens
  before the lifetime check.

## The viewer's frame loop

The browser doesn't change the tick rate. `web/arena.js` runs on `requestAnimationFrame`. It
adds `min(frame_dt, 0.1 s) * 60 * speed` to a fractional tick counter and calls
`WasmMatch.step(n)` with the whole ticks owed. So 1x plays 60 ticks per real second whatever
the display refresh rate. The 0.1 s cap stops a backgrounded tab from fast-forwarding.
