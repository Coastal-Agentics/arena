# engine-cli

Source: `engine-cli/src/main.rs`. A headless runner. It plays N duels and prints one JSON
summary to stdout. CI runs it as the smoke test (`cargo run -p engine-cli -- --matches 10
--seed 42` in the `test` job).

## What it runs

Every match is `MatchConfig::duel()` (see [world](world.md#match-config)) with:

- team 0 (tank 0): `Chaser`;
- team 1 (tank 1): `Wanderer::new(seed ^ 0x5eed)`;
- match `i` (0-based) using seed `seed.wrapping_add(i)`.

So `--matches 1 --seed S+i` reproduces match `i` of a larger run on its own (test
`match_i_is_reproducible_alone`). The policies and config are fixed; there are no flags to
change them.

## Flags

From `cargo run -p engine-cli -- --help`:

```
Coastal Agentics Arena: run headless duels between the built-in bots and print JSON results

Usage: engine-cli [OPTIONS]

Options:
      --matches <MATCHES>        Number of matches to run [default: 10]
      --seed <SEED>              RNG seed; the same seed reproduces the same matches [default: 42]
      --replay-dir <REPLAY_DIR>  Optional directory to write one JSON replay per match (`match-<seed>.json`)
  -h, --help                     Print help
  -V, --version                  Print version
```

| Flag | Type | Default | Notes |
| --- | --- | --- | --- |
| `--matches` | `u32` | 10 | `0` is allowed and prints an empty `results` list |
| `--seed` | `u64` | 42 | Decimal, 0 to 18446744073709551615. Negative or non-numeric input is rejected by clap (exit code 2) |
| `--replay-dir` | path | none | Created if missing (`create_dir_all`). Writes `match-<seed>.json` per match in [replay format 3](replay-format.md), overwriting existing files |
| `-V`, `--version` | | | Prints `engine-cli 0.1.0` (the workspace version) |

There are no subcommands. The binary and crate keep the name `engine-cli` (ADR-011 keeps crate names). The first line of `--help` is the clap `about` string in `engine-cli/src/main.rs`; test `help_uses_current_branding` keeps "Starscream" out of it.

## Output

One line of compact JSON on stdout:

```jsonc
{
  "matches": 3,               // --matches
  "seed": "42",               // --seed, as a decimal string
  "results": [
    {
      "seed": "42",           // this match's seed, decimal string
      "winner": 1,            // team: 0 = Chaser, 1 = Wanderer, null = draw
      "ticks": 447,           // ticks until the end (60 per second)
      "reason": "last_standing", // or "all_destroyed", "tick_limit"
      "hash": "03722b5e86d38fac" // final Match::state_hash, 16 hex digits
    }
    // ...
  ]
}
```

Seeds are strings so JavaScript can read them exactly ([JS-safe seeds](determinism.md#js-safe-seeds)).
The same arguments always print the same bytes (test `same_seed_same_json`).

Errors creating the directory or writing a replay go to stderr as `engine-cli: <message>` and
exit with code 1. Nothing goes to stdout in that case.

## Examples

These were run on 2026-09-30 against this branch, and the output is copied verbatim.

Three matches from seed 42:

```console
$ cargo run -q -p engine-cli -- --matches 3 --seed 42
{"matches":3,"seed":"42","results":[{"seed":"42","winner":1,"ticks":447,"reason":"last_standing","hash":"03722b5e86d38fac"},{"seed":"43","winner":1,"ticks":390,"reason":"last_standing","hash":"3810205f17326f83"},{"seed":"44","winner":1,"ticks":303,"reason":"last_standing","hash":"7f13b94a0d89d47d"}]}
```

Seeds wrap at `u64::MAX`:

```console
$ cargo run -q -p engine-cli -- --matches 2 --seed 18446744073709551615
{"matches":2,"seed":"18446744073709551615","results":[{"seed":"18446744073709551615","winner":1,"ticks":532,"reason":"last_standing","hash":"f1d983e88de5d020"},{"seed":"0","winner":1,"ticks":486,"reason":"last_standing","hash":"71597cc826ba644e"}]}
```

Write replays:

```console
$ cargo run -q -p engine-cli -- --matches 2 --seed 100 --replay-dir /tmp/replays
{"matches":2,"seed":"100","results":[{"seed":"100","winner":1,"ticks":398,"reason":"last_standing","hash":"64d7a70312c4a5db"},{"seed":"101","winner":1,"ticks":274,"reason":"last_standing","hash":"baf3fcb2cbb76c06"}]}
$ ls /tmp/replays
match-100.json
match-101.json
```

`match-101.json` has `"format":3`, `"seed":"101"`, 274 entries in `actions`, outcome
`{"winner":1,"ticks":274,"reason":"last_standing"}`, `"final_hash":"baf3fcb2cbb76c06"` (the
same hash as the summary line) and `"setup_hash":"9cfd58498bbe3f85"`.

The seed 42 result (Wanderer wins at tick 447) is the same match the web viewer shows at
`arena.html?seed=42` with Chaser vs Wanderer.

For scale, a release build ran 1000 matches (`--matches 1000 --seed 0`) in about 0.2 s of
wall time on the build box. That's one measurement, not a benchmark.
