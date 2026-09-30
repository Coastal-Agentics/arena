# Replay format (version 2)

Source: `engine/src/replay.rs`, `engine/src/json_u64.rs`, the serde derives in
`engine/src/sim.rs`, `policy.rs` and `arena.rs`, and the replay tests in `engine/src/lib.rs`
and `engine-cli/src/main.rs`.

A replay is **config + seed + every tick's actions**. It stores no positions or frames:
playing a replay means re-simulating it, so a replay is both viewer input and a determinism
check. Writing and reading use `serde_json` on the `Replay` struct; there is no separate
schema.

## Top level

Written by `Replay::to_json()` as compact JSON, one object:

| Field | JSON type | Rust type | Required on read | Meaning |
| --- | --- | --- | --- | --- |
| `format` | integer | `u32` | yes | `REPLAY_FORMAT`, currently `2` |
| `engine_version` | string | `String` | yes | `engine` crate version of the writer, e.g. `"0.1.0"`. Informational: never checked |
| `seed` | string (number accepted) | `u64` | yes | Match seed as a decimal string, e.g. `"18446744073709551615"`. See [JS-safe seeds](determinism.md#js-safe-seeds) |
| `config` | object | `MatchConfig` | yes | Full match config, below |
| `actions` | array of arrays | `Vec<Vec<Action>>` | yes | One entry per simulated tick; each entry is indexed by tank id |
| `outcome` | object or `null` | `Option<Outcome>` | no (missing = `null`) | Outcome when recorded; `null` if recorded mid-match |
| `final_hash` | string | `String` | yes | `Match::state_hash()` after the last recorded tick, 16 lowercase hex digits |

Unknown fields are ignored on read (no `deny_unknown_fields`).

## `config`

```jsonc
"config": {
  "arena": {
    "size": [800.0, 600.0],                      // Vec2 = [x, y]
    "obstacles": [                               // optional on read, default []
      { "min": [250.0, 200.0], "max": [300.0, 400.0] },
      { "min": [500.0, 200.0], "max": [550.0, 400.0] }
    ]
  },
  "tanks": [                                     // index = tank id
    { "team": 0, "pos": null, "heading": null }, // pos: [x, y] or null; heading: 0..65535 or null
    { "team": 1, "pos": null, "heading": null }  // both optional on read, default null (= random)
  ],
  "params": {
    "radius": 16.0, "max_speed": 120.0, "turn_rate": 364, "turret_turn_rate": 546,
    "max_hp": 100, "fire_cooldown": 45, "projectile_speed": 360.0, "projectile_ttl": 120,
    "projectile_damage": 20, "projectile_spread": 256
  },
  "max_ticks": 7200
}
```

(This is `MatchConfig::duel()` as `engine-cli` writes it, with comments added.) Types:

| Path | JSON | Rust |
| --- | --- | --- |
| `arena.size`, `obstacles[].min`, `obstacles[].max`, `tanks[].pos` | `[number, number]` | `glam::Vec2` (`f32`) |
| `tanks[].team` | integer 0–255 | `u8` |
| `tanks[].heading` | integer 0–65535 or `null` | `Option<u16>` (BAU) |
| `params.radius`, `max_speed`, `projectile_speed` | number | `f32` |
| `params.turn_rate`, `turret_turn_rate`, `projectile_spread` | integer 0–65535 | `u16` |
| `params.max_hp`, `projectile_damage` | integer | `i32` |
| `params.fire_cooldown`, `projectile_ttl`, `max_ticks` | integer | `u32` |

All `params` fields and `max_ticks` are required. What they mean: [world](world.md).

## `actions`

```jsonc
"actions": [
  [ {"throttle":1.0,"turn":-1.0,"turret_turn":-1.0,"fire":false},    // tick 1, tank 0
    {"throttle":0.8,"turn":-1.0,"turret_turn":-1.0,"fire":false} ],  // tick 1, tank 1
  …
]
```

- `actions.len()` is the number of ticks simulated, equal to `outcome.ticks` for a finished
  match.
- Each tick's array has one entry per tank, as written by `Match::step`. On read, a shorter
  array means the missing tanks do nothing that tick, and extra entries are ignored.
- All four `Action` fields are required: `throttle`, `turn`, `turret_turn` (numbers, `f32`),
  `fire` (bool).
- Values are stored **after** clamping (`[-1, 1]`, NaN → 0), so they're exactly what the sim
  used. A hand-edited out-of-range value is clamped again on playback.
- A dead tank's entry is whatever the caller passed. `step_policies`/`run` pass
  `Action::default()` for dead tanks, so replays from `engine-cli` hold zeros there.

## `outcome`

```json
{"winner": 1, "ticks": 276, "reason": "last_standing"}
{"winner": null, "ticks": 7200, "reason": "tick_limit"}
```

`winner`: team id or `null` for a draw. `ticks`: tick the match ended on. `reason`:
`"last_standing"`, `"all_destroyed"` or `"tick_limit"` (snake_case `EndReason`).

## Reading, playing, verifying

| API | Does |
| --- | --- |
| `Replay::from_match(&m)` / `m.replay()` | Snapshot a match, finished or not |
| `Replay::to_json()` | Compact JSON string |
| `Replay::from_json(s)` | Parse; `ReplayError::Json(msg)` on bad JSON or a missing or ill-typed field; `ReplayError::Format(n)` if `format != 2`. Doesn't simulate |
| `Replay::play()` | `Match::new(config, seed)`, then `step` each recorded tick; returns the match. No checks |
| `Replay::verify()` | `play()`, then compare outcome (`ReplayError::OutcomeMismatch`) and `final_hash`, as an exact string compare (`ReplayError::HashMismatch`). Returns the re-simulated `Match` on success |
| `ReplayPlayer::new(r)`, `.step()`, `.state()`, `.is_finished()` | Tick-by-tick playback for a viewer; `step()` returns `false` once every recorded tick is applied |

What `verify` does and doesn't prove:

- It catches any change to the seed, actions or params that changes the final state or outcome
  (`tampered_replay_is_detected` edits 200 ticks of actions and expects an error).
- It compares only the final outcome and hash. The hash doesn't cover the config
  ([state hash](determinism.md#state-hash)), so a config edit that happens not to change what
  happens passes. Example, tried on seed 7: deleting `arena.obstacles` still verified, because
  nothing in that match touched them.
- `final_hash` must be lowercase; an uppercased copy fails with `HashMismatch`.
- Ticks recorded after the match ended would be no-ops. `Match::step` never records them.

There's no CLI command that loads or verifies a replay. It's a library call (the `engine-cli`
test `written_replay_verifies` does it for a file written by `--replay-dir`), and the web
viewer doesn't load replays yet.

## Versioning

`REPLAY_FORMAT` is "bumped whenever the replay format or sim semantics change incompatibly":
a replay only means something to a sim that steps the same way. `from_json` accepts **only** the
current value. There's no migration, and `engine_version` isn't consulted.

- **v2** (current): swept projectile collision and simultaneous tank movement (so v1
  replays would no longer reproduce), and `seed` written as a decimal string. Numeric seeds
  are still read.
- **v1**: rejected with `ReplayError::Format(1)`.

Size: the seed 7 duel (276 ticks, 2 tanks) is 33,625 bytes, about 120 bytes per tick.
