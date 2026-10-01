# Replay format (version 4)

Source: `engine/src/generic/replay.rs` (the generic `Replay<R>`, `ReplayPlayer<R>`,
`setup_hash`, `ReplayError`), `engine/src/replay.rs` (the tank aliases `engine::Replay` and
`engine::ReplayPlayer`), `TankRules::check_format` in `engine/src/sim.rs`,
`engine/src/json_u64.rs`, the serde derives in `engine/src/sim.rs`, `policy.rs` and
`arena.rs`, and the replay tests in `engine/src/lib.rs` and `engine-cli/src/main.rs`.

A replay is **config + seed + every tick's actions**. It stores no positions or frames:
playing a replay means re-simulating it, so a replay is both viewer input and a determinism
check. Writing and reading use `serde_json` on the `Replay` struct; there is no separate
schema. Format 4 (current) lets the config carry per-tank params (`tanks[].params`) and
stationary accuracy (`projectile_spread_still`). Format 3 added `setup_hash`, which lets
`verify` catch edits to the seed or config. Format 2 and 3 files still load
([versioning](#versioning)).

## Top level

Written by `Replay::to_json()` as compact JSON, one object:

| Field | JSON type | Rust type | Required on read | Meaning |
| --- | --- | --- | --- | --- |
| `format` | integer | `u32` | yes | `REPLAY_FORMAT`, currently `4`. `2` and `3` are also read |
| `engine_version` | string | `String` | yes | `engine` crate version of the writer, e.g. `"0.1.0"`. Informational: never checked |
| `seed` | string (number accepted) | `u64` | yes | Match seed as a decimal string, e.g. `"18446744073709551615"`. See [JS-safe seeds](determinism.md#js-safe-seeds) |
| `config` | object | `MatchConfig` | yes | Full match config, below |
| `actions` | array of arrays | `Vec<Vec<Action>>` | yes | One entry per simulated tick; each entry is indexed by tank id |
| `outcome` | object or `null` | `Option<Outcome>` | no (missing = `null`) | Outcome when recorded; `null` if recorded mid-match |
| `final_hash` | string | `String` | yes | `Match::state_hash()` after the last recorded tick, 16 lowercase hex digits |
| `setup_hash` | string | `Option<String>` | yes in formats 3 and 4; absent in format 2 | `replay::setup_hash(seed, &config)`, 16 lowercase hex digits. See [setup hash](#setup-hash) |

Fields are written in this order. A seed 7 duel ends like this:
`…,"outcome":{"winner":1,"ticks":276,"reason":"last_standing"},"final_hash":"51234f61b02b5784","setup_hash":"0b24ce74f45e9a27"}`.

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

(This is `MatchConfig::duel()` as `engine-cli` writes it, with comments added.)

Two optional fields are new in format 4. Both are **left out when unset**, so a config
without them is written byte-for-byte as in format 3:

```jsonc
"tanks": [
  { "team": 0, "pos": null, "heading": null },
  { "team": 1, "pos": null, "heading": null,
    "params": {                                  // per-tank params: a full TankParams
      "radius": 16.0, "max_speed": 90.0, "turn_rate": 273, "turret_turn_rate": 546,
      "max_hp": 120, "fire_cooldown": 45, "projectile_speed": 360.0, "projectile_ttl": 120,
      "projectile_damage": 24, "projectile_spread": 256,
      "projectile_spread_still": 128             // optional here too
    } }
],
"params": { …, "projectile_spread": 256, "projectile_spread_still": 128 }
```

A per-tank `params` object replaces the shared one for that tank as a whole, and its `radius`
is ignored ([per-tank params](world.md#per-tank-params)). Types:

| Path | JSON | Rust |
| --- | --- | --- |
| `arena.size`, `obstacles[].min`, `obstacles[].max`, `tanks[].pos` | `[number, number]` | `glam::Vec2` (`f32`) |
| `tanks[].team` | integer 0–255 | `u8` |
| `tanks[].heading` | integer 0–65535 or `null` | `Option<u16>` (BAU) |
| `params.radius`, `max_speed`, `projectile_speed` | number | `f32` |
| `params.turn_rate`, `turret_turn_rate`, `projectile_spread` | integer 0–65535 | `u16` |
| `params.max_hp`, `projectile_damage` | integer | `i32` |
| `params.fire_cooldown`, `projectile_ttl`, `max_ticks` | integer | `u32` |
| `params.projectile_spread_still` (format 4) | integer 0–65535, `null` or absent | `Option<u16>` |
| `tanks[].params` (format 4) | object (same fields as `params`), `null` or absent | `Option<TankParams>` |

All `params` fields except `projectile_spread_still` are required, in `params` and in every
`tanks[].params` object; so is `max_ticks`. What they mean: [world](world.md).

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

## Setup hash

`engine::replay::setup_hash(seed, &config) -> u64` is a 64-bit FNV-1a over:

1. the seed's 8 bytes, little-endian, then
2. `serde_json::to_vec(&config)`: the config's **canonical** compact JSON, with fields in
   struct order and floats in `serde_json`'s shortest round-trip form.

It is computed from the config **as parsed**, not from the file's text. So every config field
is covered (including fields added later). Re-indenting a file, writing `16` for `16.0`, or
leaving out an optional `"pos": null` doesn't change it. Any change to a value does. Test
`setup_hash_ignores_json_formatting` checks the first part, and
`edited_config_fails_verification` the second.

The format 4 fields are covered the same way: `per_tank_params_replay_roundtrip_and_verify`
edits a per-tank `max_hp`, a per-tank damage and a per-tank `projectile_spread_still`, and
adds and removes a spawn's `params`, and expects `SetupMismatch` every time. Because unset
fields are left out of the serialized config, **a config without them hashes exactly as it
did in format 3**: every setup hash quoted in these pages is unchanged (tests
`documented_hashes_are_unchanged` in `games/tank` and `pinned_hashes_are_unchanged` in
`engine`). Adding a spawn's `params` equal to the shared set
changes the setup hash, even though the match plays the same.

`Match::state_hash()` (the `final_hash` and `engine-cli`'s `hash`) is **unchanged** by this.
It still covers only the dynamic state ([state hash](determinism.md#state-hash)).

## Reading, playing, verifying

| API | Does |
| --- | --- |
| `Replay::from_match(&m)` / `m.replay()` | Snapshot a match, finished or not |
| `Replay::to_json()` | Compact JSON string |
| `Replay::from_json(s)` | Parse, then check in this order: `ReplayError::Json(msg)` on bad JSON or a missing or ill-typed field; `ReplayError::Format(n)` unless `format` is 2, 3 or 4; `ReplayError::MissingSetupHash` for format 3 or 4 without `setup_hash`; `ReplayError::FieldNotInFormat { format, field }` for a format 2 or 3 file that uses a format 4 field (the rules' `check_format`; for tanks, `TankRules::check_format`). Doesn't simulate |
| `Replay::play()` | `Match::new(config, seed)`, then `step` each recorded tick; returns the match. No checks |
| `Replay::verify()` | 1. if `setup_hash` is present, recompute it from `seed` and `config` (`ReplayError::SetupMismatch`); 2. `play()`; 3. compare the outcome (`ReplayError::OutcomeMismatch`); 4. compare `final_hash` (`ReplayError::HashMismatch`). Hashes are exact string compares. Returns the re-simulated `Match` on success |
| `ReplayPlayer::new(r)`, `.step()`, `.state()`, `.is_finished()` | Tick-by-tick playback for a viewer; `step()` returns `false` once every recorded tick is applied |

What `verify` does and doesn't prove:

- **Seed and config (formats 3 and 4):** any edit to `seed` or to any value in `config` fails with
  `SetupMismatch`, even if the match would play out the same. The regression test
  `edited_config_fails_verification` deletes `arena.obstacles` from the seed 7 duel, confirms
  the edited replay re-simulates to the same outcome and state hash, and expects
  `SetupMismatch`. In format 2 this edit verified. The same test covers arena size, a tank
  param, spread, a spawn heading, a team and `max_ticks`, and the seed.
- **Actions:** any change that alters the final state or outcome fails
  (`tampered_replay_is_detected` edits 200 ticks and expects an error). An action edit that
  changes nothing, e.g. to a dead tank's entry or an out-of-range value that clamps to the
  same number, passes. It isn't hashed.
- **Not a signature.** Anyone editing a file can recompute both hashes. They catch accidents,
  corruption and sim drift, not deliberate forgery.
- Hashes must be lowercase; an uppercased copy fails.
- Format 2 replays have no `setup_hash`, so their seed and config aren't checked: deleting
  the obstacles from a v2 file of the seed 7 duel still verifies.
- Ticks recorded after the match ended would be no-ops. `Match::step` never records them.

There's no CLI command that loads or verifies a replay. It's a library call (the `engine-cli`
test `written_replay_verifies` does it for a file written by `--replay-dir`), and the web
viewer doesn't load replays yet.

## Versioning

`REPLAY_FORMAT` is "bumped whenever the replay format or sim semantics change incompatibly":
a replay only means something to a sim that steps the same way. `from_json` accepts
`OLDEST_READABLE_FORMAT` (2) through `REPLAY_FORMAT` (4). `engine_version` isn't consulted.
Every accepted format keeps its `format` when read and written back; `to_json()` never
upgrades a file. To upgrade any older replay, verify it and re-record:
`Replay::from_json(s)?.verify()?.replay()` is a format 4 replay of the same match.

- **v4** (current, 2026-09-30, Tank Arena spec engine ask #5): the config may carry
  `tanks[].params` and `projectile_spread_still` (in `params` or a spawn's `params`). A config
  without them **plays exactly as in v2 and v3**, serializes to the same bytes and has the same
  setup hash. So a replay written today for such a config differs from its v3 twin only in
  `"format":4`. Why bump at all: a v3 reader would ignore the new fields it doesn't know
  (unknown fields are ignored) and play a different match.
- **v3** (2026-09-30): adds `setup_hash`. **Sim semantics are unchanged from v2**,
  so the same seed, config and actions give the same state and `final_hash` as before.
  **Still read**, with the setup hash checked. `from_json` rejects a v3 file that uses a
  format 4 field (`ReplayError::FieldNotInFormat`, e.g. `field: "tanks[].params"` or
  `"params.projectile_spread_still"`); `null` counts as unset. Test:
  `format_3_replays_still_load_and_verify`.
- **v2**: swept projectile collision and simultaneous tank movement (so v1 replays would no
  longer reproduce), and `seed` written as a decimal string (numeric seeds still read).
  **Still read:** a v2 file loads with `format: 2` and `setup_hash: None`, verifies on
  outcome and `final_hash` only, and `to_json()` writes it back as v2 (no `setup_hash`). The
  format 4 fields are rejected as for v3. Test: `format_2_replays_still_load_and_upgrade`.
- **v1, and anything above 4**: rejected with `ReplayError::Format(n)`.

No replays are committed in this repo or on `nightly-data` (checked 2026-09-30), so nothing
needed converting. The 20 v3 replays `engine-cli --matches 20 --seed 0 --replay-dir` wrote
from `main` before this change all load and verify with it.

Size: the seed 7 duel (276 ticks, 2 tanks) is 33,657 bytes in v4 and v3 (v2 was 33,625;
`setup_hash` adds 32), about 120 bytes per tick. A spawn's `params` with the default values adds 207 bytes (more with `projectile_spread_still`).
