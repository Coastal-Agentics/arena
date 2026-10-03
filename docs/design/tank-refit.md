# Tank refit to the shared interface: constraints

**Status:** Draft for review (2026-10-03).
**Author:** Blitzwing (Tank Designer-Developer). Part of [game-system.md](game-system.md) §2 and milestone M2 (Shockwave).
**In one line:** Tank Arena becomes game #1 under the shared interface by *adding* three things. Nothing a player, a replay or the nightly can see changes.

## What the refit adds
| Addition | Owner | What it is |
|---|---|---|
| Flat view: `tank::encode_obs` / `decode_action` | Blitzwing | `OBS_LEN = 176`, the layout already agreed in GATE-003 §6 (self 11, enemies and allies 96, shells 48, walls 4, tick 1, obstacles 16; every value clamped to [−1, 1]; cos and sin from the engine's table). `ACTION_LEN = 4`: throttle, turn, turret turn, and fire = value > 0. |
| Tank `reward` | Blitzwing | The GATE-003 §6 proposal, computed from state and the step's events: see below. |
| E2: reusable scratch buffers in `TankRules` | Shockwave | Same iteration order, same tie order and the same float operations. Speed equal or better. |

**Tank reward, per agent per tick:**
- **Shaping:** for each `Event::Hit` this tick, the tank that hit gets +0.5 × damage ÷ the target's max HP, and the tank that was hit gets −0.5 × damage ÷ its own max HP. Over a duel this adds at most ±0.5, and it is zero-sum.
- **Terminal reward:** on the tick the match ends with a `Destroyed` event, tanks still in play get +1 if their team won and −1 if it lost. A wipe-out is a draw and gets 0.
- **At the 7,200-tick cap:** the match is a draw (*truncated*), and the terminal reward is 0.
- **Never recorded:** the reward is never stored in replays, and evolution keeps its match-result fitness (win 1, draw 0.25, loss 0).

**The rich `Observation` stays as it is,** `Vec`s and all, for the Rust policies. Moving it to fixed arrays is *not* part of the refit. The way the arrays are filled and sorted feeds every policy decision, so a small difference could move the M1 champion's hashes. If we ever do it, it is a separate PR under the same checks as below.

## What must not change
| Item | Pinned value (on `main` at `40389cb`) | Where it's checked |
|---|---|---|
| Chaser vs Wanderer pins | seed 42: tick 447, `03722b5e86d38fac` · seed 7: 276, `51234f61b02b5784`, setup `0b24ce74f45e9a27` · seed 101: 274, `baf3fcb2cbb76c06`, setup `9cfd58498bbe3f85` · seed u64::MAX: 532, `f1d983e88de5d020` | `games/tank/src/bots.rs` `documented_hashes_are_unchanged` |
| Bot smoke run, seeds 0–199 | digest `28ae434ec1996a74` | `smoke_run_seeds_0_to_199_are_unchanged` |
| CLI smoke output | `engine-cli --matches 10 --seed 42` byte-identical (first row: seed 42, winner 1, 447 ticks, `03722b5e86d38fac`) | CI `test` job |
| Parity fixtures (7) | `cw-seed-max` `f1d983e88de5d020` · `cw-all-destroyed` `fc9b0fbc0058d761` · `cw-per-tank-params` `86bc2f990b3b015d` · `cw-spread-still` `92981daa6ab1c79f` · `arena-charger-mirror` `d8b3fec9b177e010` · `arena-sniper-vs-charger` `e09cf50fd8e2160b` · `arena-2v2-tick-limit` `0aa1ae238e20374d` | `engine-wasm/tests/parity/`, `scripts/check-parity.mjs`, CI `wasm` job |
| M1 pinned champion | `charger-2-5-2`, Gen 99, 5,300 of 6,000 (88.3%), digest `d15709d4b3bd6953`, experimental | `games/tank/tests/evolve_m1.rs` (plus the slow `--ignored` full run) |
| Balance | Kiter > Charger 69.2%, Charger > Sniper 64.5%, Sniper > Kiter 75.0%; median match 44.6 s; draws 8.3% | `cargo run -p tank --release --example balance -- 200` regenerates `BALANCE.md` with no diff |
| Replays | `REPLAY_FORMAT` 4, `OLDEST_READABLE_FORMAT` 2, `setup_hash`, the `MatchConfig` and `Action` JSON shapes | engine replay tests; parity fixtures |
| Evolution | gene tables (Charger 14, Kiter 15, Sniper 20), dodge cap 0.65 / 0.95 / 0.61, GA config, fitness | genome tests; `evolve verify` on the `nightly-data` lineage |
| Old URLs | `?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`, the legacy `?seed=…&a=Chaser&b=Wanderer`, and `tab`, `speed`, `paused`, `t`; canonical query output | `scripts/check-viewer.mjs`, `scripts/check-viewer-browser.py` |

**The replay format stays 4 and byte-identical.** The refit writes exactly the files it writes today. Format 5 arrives later with racing, and a format 4 file with no `game` field keeps meaning Tank Arena. Whether tank ever writes format 5 is a separate decision. It would change the file bytes but not `final_hash` or `setup_hash`.

## How each refit PR proves it
1. **All green, as CI runs them:**
   - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`;
   - `cargo test -p tank --release --test evolve_m1 -- --ignored`;
   - `./scripts/build-wasm.sh` (the `web/pkg` bytes may change; behaviour must not);
   - `node scripts/check-viewer.mjs`, `node scripts/check-parity.mjs` and `scripts/check-viewer-browser.py`.
2. **No diff** when `BALANCE.md` is regenerated, and `evolve verify` replays the current nightly champion exactly.
3. **A before/after table in the PR body.** It has one row per item in "What must not change", main's value next to the branch's value, and every row identical. Speed comes from `engine-cli --matches 200 --seed 0`, release, median of 5 runs, as in game-system.md §1.
4. **New tests for the additions:**
   - `encode_obs` has length 176, every value is in [−1, 1], it handles the tie-break and overflow cases, and it gives the same output natively and in wasm;
   - `decode_action` round-trips;
   - over a match, the shaping rewards sum to zero, and the terminal reward has the sign of `Outcome.winner`.

## What the refit may change
- New code: the `Flat` impl, `reward`, E2's buffers, and the wasm export used for the Customize schema ([viewer-multi-game.md](viewer-multi-game.md)).
- Internal layout: where code sits and how buffers are reused, and the `web/pkg` bytes.
- Docs, including SPEC.md, which gains a short "Flat encoding and reward" section.
- *Not* the rules, policy params, tables, observations, actions, JSON shapes or anything in the table above. Any of those is a GATE-002 or GATE-003 amendment, with its own PR and a re-pin.

If Nye approves B2–B5 (game-system.md M5), moving `TankRules` and the tank types into `games/tank` is a *verbatim* move under exactly these checks. It is a separate PR from the refit.

## Milestones and acceptance (game side)
| | What | Accepted when |
|---|---|---|
| **T1** (in M2) | `tank::encode_obs` + `decode_action` | The new tests above pass; nothing in the table moves. |
| **T2** (in M2) | Tank `reward` | The reward tests pass; nothing in the table moves. |
| **T3** (with M5, if approved) | B4: tank rules into `games/tank` (Blitzwing), with Shockwave's B2, B3 and B5 | Every row of the before/after table is identical. |
