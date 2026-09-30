# Seeds and determinism

Source: `engine/src/sim.rs`, `engine/src/angle.rs`, `engine/src/arena.rs`,
`engine/src/json_u64.rs`, `engine/src/bots.rs`; policy in ADR-003 (`docs/DECISIONS.md`).

## The guarantee

**Same config + same seed + same actions → bit-identical state on the same platform.**
`Match::state_hash()` is how that is checked. Cross-platform bit-identity is a stretch goal
(ADR-003), not a promise.

As a spot check on 2026-09-30, Chaser vs Wanderer for seeds 0, 7, 42, 43, 1234 and
18446744073709551615 gave the same tick count, outcome and final hash in the wasm build
(headless Chrome) as `engine-cli` on native x86_64 Linux. That's evidence for those seeds on
those two targets, nothing more.

Tests that pin this down: `same_seed_same_match`, `different_seeds_differ`,
`replay_json_roundtrip_reproduces_match`, `tampered_replay_is_detected` (engine),
`same_seed_same_json`, `match_i_is_reproducible_alone` (engine-cli) and
`chaser_vs_wanderer_matches_headless_run` (engine-wasm, run natively).

## The seeded RNG

Each `Match` owns one `rand_chacha::ChaCha8Rng`, created with `ChaCha8Rng::seed_from_u64(seed)`.
`rand_chacha` is built with `default-features = false`, so no OS entropy (`getrandom`) is
linked. That is also what lets the crate build for `wasm32-unknown-unknown`. Nothing in
the sim uses a global or thread RNG.

### What draws from the match RNG

In this order, and nowhere else:

1. **`Match::new`, per tank in id order:**
   - if `pos` is `None`: up to 1000 candidates, each drawing two `u32`s (x then y). Each is
     turned into a float in `[0, 1)` from its top 24 bits and scaled to `[r, size - r]`. A
     candidate is accepted if no obstacle is within 1.5 radii and every already-placed tank's
     centre is at least 6 radii away. If all 1000 fail, the tank is placed at the arena
     centre;
   - if `heading` is `None`: one `u32`, top 16 bits → heading.
2. **`Match::step`, per shot fired, in tank id order** (only if `projectile_spread > 0`):
   one `u32`; deviation = `u32 % (2 * spread + 1) - spread` BAU.

Because shots draw in tank id order, the RNG stream depends on the order of `config.tanks`.
Movement does not (see below).

### Policies bring their own randomness

The `Policy` trait asks policies to be deterministic given their own state and the
observations; a policy that wants randomness keeps its own seeded RNG. `Wanderer::new(seed)`
does exactly that (its own `ChaCha8Rng`). By convention:

- `engine-cli` seeds team 1's Wanderer with `match_seed ^ 0x5eed`;
- the web viewer does the same for a Wanderer on team 1, and uses `match_seed ^ 0x5eed_0000`
  for a Wanderer on team 0 (so Wanderer vs Wanderer doesn't mirror).

That's why Chaser (blue) vs Wanderer (orange) in the viewer reproduces `engine-cli --seed S`.

A replay stores actions, not policies, so it re-simulates without any policy RNG.

## Where float drift could come from, and what the engine does about it

- **No platform trig.** Headings are integer BAU; `sin`/`cos`/`dir` read a compile-time table
  built with `+ - * /` only (`angle.rs`). Aiming (`angle::turn_toward`) uses a cross and dot
  product against a tolerance, not `atan2`.
- **Squared distances** for overlap tests.
- **One `sqrt`**, in the swept projectile-vs-tank test (`arena::segment_circle_entry`).
  IEEE-754 requires `sqrt` to be correctly rounded, so it gives the same bits everywhere.
  ADR-003 says to avoid `sqrt` "where avoidable"; this is the one place the code chose to use it.
- **Plain `f32` arithmetic** otherwise (via `glam::Vec2`).
- **Deterministic order everywhere:** `Vec`s iterated by index, no hash maps. Observation
  lists are sorted with `f32::total_cmp` (enemies and allies tie-break by id; projectiles use a
  stable sort, so ties keep list order).

## Simultaneous movement

Tank moves are planned against the **tick-start** positions of every tank, then applied
together. Any tank whose planned position would overlap another's planned position stays put,
repeated until stable ([tick loop](tick-loop.md#inside-one-step), step 2). No tank moves
"first", so the movement result doesn't depend on tank ids. Engine test
`movement_does_not_depend_on_tank_id_order` checks this. Before replay format 2, moves were
applied in id order (see [replay format](replay-format.md#versioning)).

## State hash

`Match::state_hash()` is a 64-bit FNV-1a over the raw bits of:

- the tick count;
- per tank, in id order: `pos.x`, `pos.y`, `vel.x`, `vel.y`, `heading`, `turret`, `hp`,
  `cooldown`, `alive`;
- per projectile, in list order: `owner`, `pos.x`, `pos.y`, `vel.x`, `vel.y`, `ttl`.

Floats are hashed by `to_bits()`, so any drift at all changes the hash. It does **not** cover
the config, the seed, the RNG's internal state, events or the action history. Replays and
`engine-cli` print it as 16 lowercase hex digits (`format!("{:016x}")`).

## JS-safe seeds

Seeds are `u64`, and JavaScript numbers lose integer precision above 2^53. So everywhere the
engine writes a seed to JSON, it writes a **decimal string**, via `engine::json_u64`
(`#[serde(with = "engine::json_u64")]`):

- writing: `"seed":"18446744073709551615"`;
- reading: a decimal string **or** a non-negative JSON integer (older replays and hand-written
  JSON used numbers). Negative numbers, fractions and non-numeric strings are errors.

Used by `Replay::seed` and by both `seed` fields in `engine-cli`'s summary. Hashes are also
strings (hex). In the browser, `new WasmMatch(seed, …)` takes the seed as a string and parses
it with Rust's `u64` parser after trimming whitespace, so `"-1"` or `"abc"` throw.

`engine-cli` derives match `i`'s seed as `seed.wrapping_add(i)`, so `--seed 18446744073709551615
--matches 2` runs seeds 18446744073709551615 and 0.
