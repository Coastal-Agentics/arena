# Seeds and determinism

Source: `engine/src/generic/` (`MatchRng`, `StateHasher`, the match loop), the parity check
(`engine-wasm/tests/parity/`, `engine-wasm/tests/parity.rs`, `scripts/check-parity.mjs`),
`engine/src/sim.rs` (`TankRules`), `engine/src/angle.rs`, `engine/src/arena.rs`,
`engine/src/json_u64.rs`, `games/tank/src/bots.rs` (the placeholder bots); policy in ADR-003
(`docs/DECISIONS.md`).

## The guarantee

**Same config + same seed + same actions → bit-identical state on the same platform.**
`Match::state_hash()` is how that is checked. Cross-platform bit-identity is a stretch goal
(ADR-003), not a promise.

As a spot check on 2026-09-30, Chaser vs Wanderer for seeds 0, 7, 42, 43, 1234 and
18446744073709551615 gave the same tick count, outcome and final hash in the wasm build
(headless Chrome) as `engine-cli` on native x86_64 Linux. So did a per-tank loadout through
`WasmMatch.withConfig` (Glass Cannon 28 dmg / 60 HP vs Brawler 24 dmg, 90 u/s, 273 BAU/tick,
120 HP, seed 42: tick 385, hash `86bc2f990b3b015d`). That's evidence for those runs on those
two targets, nothing more. The same runs gave the same results again after the bots moved
from `engine::bots` to `games/tank` (ADR-014 Phase A, 2026-09-30), and again after the
generic core (ADR-014 B1, 2026-09-30), which also left `engine-cli` output and 200 replay
files byte-identical.

Tests that pin this down:
- **engine:** `same_seed_same_match`, `different_seeds_differ`,
  `replay_json_roundtrip_reproduces_match`, `tampered_replay_is_detected`,
  `per_tank_params_are_deterministic_and_change_the_match`, and the engine's own pins,
  `pinned_hashes_are_unchanged` (5 seeds, including an `all_destroyed` draw) and
  `seeds_0_to_199_are_unchanged` (a digest). These run test-only policies from
  `engine/src/testing.rs`, so they don't depend on any game crate. The generic core has its
  own tests (`engine/src/generic/tests.rs`) on a small test-only second `Rules` game:
  `same_seed_same_match_different_seed_differs`, `state_hash_is_tick_then_rules_state`,
  `replays_roundtrip_verify_and_reject` and others.
- **games/tank:** `documented_hashes_are_unchanged` (the Chaser vs Wanderer hashes quoted in
  these pages) and `smoke_run_seeds_0_to_199_are_unchanged`, next to the bots.
- **engine-cli:** `same_seed_same_json`, `match_i_is_reproducible_alone`, and
  `documented_rows_are_unchanged` (the rows quoted on the engine-cli page).
- **engine-wasm:** `chaser_vs_wanderer_matches_headless_run` (run natively).

## The seeded RNG

Each `Match` owns one `rand_chacha::ChaCha8Rng`, created with `ChaCha8Rng::seed_from_u64(seed)`
and wrapped in `engine::MatchRng`. Only the engine can create a `MatchRng`, and it exposes
only `next_u32` and `next_u64`. The match lends it to exactly two `Rules` functions,
`init` and `step`; `observe`, `outcome` and the rest never see it.
`rand_chacha` is built with `default-features = false`, so no OS entropy (`getrandom`) is
linked. That is also what lets the crate build for `wasm32-unknown-unknown`. Nothing in
the sim uses a global or thread RNG.

### What draws from the match RNG

In this order, and nowhere else:

1. **`Match::new` (`TankRules::init`), per tank in id order:**
   - if `pos` is `None`: up to 1000 candidates, each drawing two `u32`s (x then y). Each is
     turned into a float in `[0, 1)` from its top 24 bits and scaled to `[r, size - r]`. A
     candidate is accepted if no obstacle is within 1.5 radii and every already-placed tank's
     centre is at least 6 radii away. If all 1000 fail, the tank is placed at the arena
     centre;
   - if `heading` is `None`: one `u32`, top 16 bits → heading.
2. **`Match::step` (`TankRules::step`), per shot fired, in tank id order**, only if that shot's effective spread
   is above 0: one `u32`; deviation = `u32 % (2 * spread + 1) - spread` BAU. The effective
   spread is the firing tank's own `projectile_spread`, or its `projectile_spread_still` when
   that is set and the tank didn't move this tick ([stationary accuracy](world.md#stationary-accuracy)).

Because shots draw in tank id order, the RNG stream depends on the order of `config.tanks`.
Movement does not (see below). The number of draws can depend on movement, though: with
`projectile_spread_still: Some(0)`, a still tank's shot draws nothing, so every later draw in
the match shifts by one. That's still deterministic (the same inputs give the same draws), just
worth knowing when comparing two matches. Per-tank params add no draws: spawns still draw
exactly as listed in step 1, with the shared radius.

### Policies bring their own randomness

The `Policy` trait asks policies to be deterministic given their own state and the
observations; a policy that wants randomness keeps its own seeded RNG. `tank::Wanderer::new(seed)`
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
  The Tank Arena spec's engine ask #1 asks for swept hits with "dot products, no sqrt". The
  swept hits it asks for have been in since PR #6 (the spec's own "Known engine limits"
  cites it), with this one `sqrt` for the entry time. It was kept: it is deterministic, and a
  `sqrt`-free rewrite would round differently in edge cases, so it could change outcomes and
  hashes.
  Line of sight (`Arena::segment_clear`) uses no `sqrt`.
- **Plain `f32` arithmetic** otherwise (via `glam::Vec2`).
- **Deterministic order everywhere:** `Vec`s iterated by index, no hash maps. Observation
  lists are sorted with `f32::total_cmp` (enemies and allies tie-break by id; projectiles use a
  stable sort, so ties keep list order).

## Simultaneous movement

Tank moves are planned against the **tick-start** positions of every tank, then applied
together. Any tank whose planned position would overlap another's planned position stays put,
repeated until stable ([tick loop](tick-loop.md#inside-one-step-tank-arena), step 2). No tank moves
"first", so the movement result doesn't depend on tank ids. Engine test
`movement_does_not_depend_on_tank_id_order` checks this. Before replay format 2, moves were
applied in id order (see [replay format](replay-format.md#versioning)).

## State hash

`Match::state_hash()` is a 64-bit FNV-1a (`engine::StateHasher`, one `write_u64` per value)
over the raw bits of:

- the tick count (written by the generic match loop);
- then whatever the rules' `hash_state` writes. For `TankRules`: per tank, in id order: `pos.x`, `pos.y`, `vel.x`, `vel.y`, `heading`, `turret`, `hp`,
  `cooldown`, `alive`;
- per projectile, in list order: `owner`, `pos.x`, `pos.y`, `vel.x`, `vel.y`, `ttl`.

Floats are hashed by `to_bits()`, so any drift at all changes the hash. It does **not** cover
the config (including per-tank params), the seed, the RNG's internal state, events or the
action history. (Replays cover
the seed and config separately, with a [setup hash](replay-format.md#setup-hash) since format
3.) Replays and
`engine-cli` print it as 16 lowercase hex digits (`format!("{:016x}")`).

## Native-vs-wasm parity

The spot checks above say the wasm build and native agree on the runs we tried. The parity
check makes that a standing test on a fixed set of pinned replays (GATE-003 ask 2):

- **Fixtures:** `engine-wasm/tests/parity/*.json`, seven format-4 replays (506,596 bytes in
  total: 120 to 140 bytes per tick for two tanks, about 270 for four), and
  `engine-wasm/tests/parity/manifest.json`. For each file the manifest gives why it exists and
  the natively recorded `format`, `seed`, `tanks`, `ticks`, `outcome`, `final_hash`,
  `setup_hash` and `bytes`.

  | File | Why | Ticks | End |
  | --- | --- | --- | --- |
  | `cw-seed-max.json` | Chaser vs Wanderer at seed `u64::MAX` (not a JS-safe number) | 532 | `last_standing` |
  | `cw-all-destroyed.json` | Chaser vs Wanderer, seed 2916: both die on one tick | 236 | `all_destroyed` |
  | `cw-per-tank-params.json` | Per-tank params, the documented Glass Cannon vs Brawler example | 385 | `last_standing` |
  | `cw-spread-still.json` | `projectile_spread_still: Some(0)`, so RNG draws depend on movement | 263 | `last_standing` |
  | `arena-charger-mirror.json` | Tank Arena v1 via `MatchSpec` (loadouts, spawns, pillars), a draw | 818 | `all_destroyed` |
  | `arena-sniper-vs-charger.json` | Tank Arena; the sniper fires only with line of sight past the pillars | 881 | `last_standing` |
  | `arena-2v2-tick-limit.json` | Tank Arena 2v2 (four tanks, teammates), all three policies, `max_ticks` cut to 360 | 360 | `tick_limit` |

- **Native side:** `engine-wasm/tests/parity.rs`. It runs in `cargo test --workspace`, so
  the CI `test` job already runs it. Every fixture must load as the current `REPLAY_FORMAT`,
  pass `Replay::verify`, and match the manifest field by field. It also checks that the
  manifest lists every fixture file, and that a tampered fixture fails.
- **Wasm side:** `node scripts/check-parity.mjs`. It loads the committed `web/pkg` with
  `initSync`, re-simulates each fixture with `checkReplayJson` ([JS API](wasm-and-web.md#js-api-from-webpkgengine_wasmjs)),
  and compares with the same manifest. It has no npm dependencies and takes about 0.2 s. On
  a mismatch it prints each differing field with the manifest (native) value and the wasm
  value, then exits 1:

  ```
  FAIL  arena-sniper-vs-charger.json
          final_hash
            manifest (native): "e09cf50fd8e2160b"
            wasm:              "f950a703522b3077"
          Replay::verify
            manifest (native): ok
            wasm:              state hash mismatch: expected e09cf50fd8e2160b, got f950a703522b3077
  check-parity: FAILED, 1 fixture(s) differ between native (manifest) and wasm (web/pkg).
  ```

  (That output is from a deliberate local edit: tank 0's throttle on tick 100 went from 1.0
  to 0.5.) The script exports `checkFixtures(manifest, texts, checkReplayJson)`, so the same
  comparison runs in a browser page. On 2026-09-30 headless Chrome 154 and Node 20 gave
  identical `checkReplayJson` output for all seven fixtures.
- **CI:** the native side runs in the `test` job, and the Node script in the `wasm` job
  (since #32, from [CI specs](ci-specs.md) (a)).

**Regenerating:** `cargo run -p engine-wasm --example parity_fixtures` rewrites every fixture
and the manifest. It's deterministic: two runs give byte-identical files. The replays store
actions, not policies, so they stay valid when a `games/tank` policy changes (a regenerated
file would just differ).

**When a fixture change is legitimate:** only with a `REPLAY_FORMAT` bump or a deliberate
change to the sim rules (movement, firing, hits, end conditions, the state hash or the
setup hash), and the PR must say so and why. Any other reason to touch these files means
native and wasm, or old and new, disagree, and that is the bug to find. Adding a new
fixture for new coverage is fine; say which case it adds.

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
