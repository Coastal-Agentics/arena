# CI specs for the workflow owner

Owner of `.github/workflows/`: Soundwave (Coastal CoS). Engine PRs don't edit workflows, so
this page gives the exact changes for the workflow owner to apply. Written 2026-09-30 by
Shockwave against the workflows on `main` at `2e899fb`:

- `ci.yml`: jobs `lint` (`ubuntu-latest`), `test` and `wasm` (both pinned to `ubuntu-24.04`
  since #29, because pinned hashes and the committed wasm are same-platform results,
  ADR-003);
- `nightly.yml`: `self-play`, pinned to `ubuntu-24.04`;
- `pages.yml`: `deploy` on `ubuntu-latest`, which uploads `web/` as-is.

Checkout is `actions/checkout@v7` and caching is `actions/cache@v6` or `Swatinem/rust-cache@v2`.
The new steps below use the same pins.

## (a) Native-vs-wasm parity hookup (applied in #32)

What it runs: `node scripts/check-parity.mjs`, the wasm side of the
[parity check](determinism.md#native-vs-wasm-parity). The native side,
`engine-wasm/tests/parity.rs`, is already in CI because `cargo test --workspace` in the `test`
job runs it. Nothing to add there.

**Where:** `ci.yml`, job `wasm` (already `runs-on: ubuntu-24.04`). Add these two steps right
after the step `web/pkg matches committed copy` and before `Browser viewer check`. The
script then checks the committed `web/pkg`, which the step before has just proved equal to
a rebuild from source.

```yaml
      - uses: actions/setup-node@v7
        with:
          node-version: "22"
          package-manager-cache: false
      - name: Native-vs-wasm parity (pinned replays)
        run: |
          node --version
          node scripts/check-parity.mjs || {
            echo "::error::web/pkg disagrees with the natively recorded manifest in engine-wasm/tests/parity/. See docs/engine/determinism.md (Native-vs-wasm parity)."
            exit 1
          }
```

- **Node version:** 22 (LTS), pinned by major so a runner image update can't change it. The
  script needs only ES modules, top-level `await` and `TextEncoder`, and was run on Node 20
  and in Chrome 154. If you'd rather not add an action, drop the `setup-node` step and use
  the image's Node; the `node --version` line records which one ran.
- **Caching:** none needed. There are no npm dependencies (so `package-manager-cache: false`),
  and the step reads only committed files. `setup-node` takes Node 22 from the runner's tool
  cache.
- **Runtime:** the script takes about 0.2 s (7 replays, 3,475 ticks). `setup-node` adds a
  few seconds, so expect under 10 s on top of the current `wasm` job.
- **Failure semantics:** the script exits 1 if any fixture fails to load, fails
  `Replay::verify` in wasm, differs from the manifest in any of `format`, `seed`, `tanks`,
  `ticks`, `outcome`, `final_hash`, `setup_hash` or `bytes`, or if a fixture file is missing
  from the manifest. The log prints each differing field as `manifest (native)` vs `wasm`,
  and the `::error::` line annotates the run. It exits 0 only when all fixtures match. A
  failure is never flaky: same inputs, same bytes. It means one of three things, and the fix
  is never to rerun:
  1. `web/pkg` behaves differently from native (a real parity bug);
  2. a fixture or the manifest was edited by hand;
  3. the sim changed without regenerating. That's legitimate only with a `REPLAY_FORMAT` bump
     or a deliberate rule change, stated in the PR.

  The `test` job's `parity` test fails in cases 2 and 3 as well, so a red `wasm` job with a
  green `test` job points at case 1.
- **Also consider (optional):** `node scripts/check-viewer.mjs` (the viewer's pure helpers
  against `web/pkg`, about 0.2 s) isn't in CI today. It fits right after the parity step.

## (b) engine-py wheel build (spec for later: M3, `engine-py/` does not exist yet)

**Do not apply this until the `engine-py/` crate lands.** It is recorded now so the M3 PR
and the workflow change can be reviewed against one plan (GATE-003 plan §6 and the M3 row;
ask 4 there).

**Keeping existing jobs unaffected.** Every current job runs `cargo … --workspace`, and
`--workspace` builds *all* members, whatever `default-members` says. So `default-members`
alone would not keep `engine-py` out of `cargo test --workspace` or `cargo clippy
--workspace`. The M3 PR should instead:

- add `exclude = ["engine-py"]` to the root `[workspace]`, and give `engine-py/Cargo.toml`
  its own empty `[workspace]` table and its own `Cargo.lock`. It depends on `engine` and
  `games/tank` by path. The existing jobs then never compile pyo3 or link Python.
- not use pyo3's `extension-module` feature. It is deprecated; maturin 1.9.4 or later sets
  `PYO3_BUILD_EXTENSION_MODULE` itself. Build with pyo3's `abi3-py310`, so one wheel covers
  CPython 3.10 to 3.14.

The alternative, gating by feature while keeping `engine-py` a member, would still make
every `--workspace` job build pyo3 and need a Python interpreter, so it isn't recommended.

**Proposed new workflow** `.github/workflows/engine-py.yml`, separate so engine CI isn't
slowed:

```yaml
name: engine-py

on:
  pull_request:
    paths: ["engine/**", "games/tank/**", "engine-py/**", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/engine-py.yml"]
  push:
    branches: [main]
    paths: ["engine/**", "games/tank/**", "engine-py/**", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/engine-py.yml"]
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: engine-py-${{ github.ref }}
  cancel-in-progress: true

jobs:
  wheel:
    name: wheel (abi3-py310)
    # Pinned like test/wasm: the parity step compares hashes (ADR-003: same platform).
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v7
      - name: Install toolchain (from rust-toolchain.toml)
        run: rustup toolchain install && rustup show
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: engine-py
      - uses: actions/setup-python@v7
        with:
          python-version: "3.10" # the abi3 floor: build once, on the oldest supported
      - name: Build one abi3 wheel
        run: |
          python -m pip install --disable-pip-version-check "maturin>=1.9.4,<2"
          maturin build --release -m engine-py/Cargo.toml --out dist
          ls -l dist
      - uses: actions/upload-artifact@v7
        with:
          name: engine-py-wheel
          path: dist/*.whl

  test:
    name: test (py${{ matrix.python }})
    needs: wheel
    runs-on: ubuntu-24.04
    strategy:
      fail-fast: false
      matrix:
        python: ["3.10", "3.14"] # the same abi3 wheel on the oldest and newest
    steps:
      - uses: actions/checkout@v7
      - name: Install toolchain (from rust-toolchain.toml)
        run: rustup toolchain install && rustup show
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-python@v7
        with:
          python-version: ${{ matrix.python }}
          cache: pip
          cache-dependency-path: engine-py/requirements-test.txt
      - uses: actions/download-artifact@v8
        with:
          name: engine-py-wheel
          path: dist
      - name: Install the wheel and test deps
        run: |
          python -m pip install --disable-pip-version-check dist/*.whl -r engine-py/requirements-test.txt
      - name: PettingZoo API and seed tests
        run: python -m pytest -q engine-py/tests
      - name: Python-vs-Rust parity (1,000-step episode)
        run: |
          python engine-py/tests/record_episode.py --steps 1000 --seed 42 --out target/py-parity/episode.json
          cargo run -q -p engine-wasm --example verify_replay -- target/py-parity/episode.json
```

What the M3 PR has to provide for this to run (none of it exists yet):

- `engine-py/requirements-test.txt` pinning `pettingzoo==1.27.0`, `pytest` and `numpy`.
- `engine-py/tests/test_pettingzoo.py` running `pettingzoo.test.parallel_api_test(env,
  num_cycles=1000)` and `pettingzoo.test.parallel_seed_test(env_fn)` on
  `TankArenaParallelEnv`.
- `engine-py/tests/record_episode.py`, which plays a Python-driven episode and writes its
  `Replay` JSON, including the `final_hash` Python saw.
- A Rust verifier outside `engine/`: an `engine-wasm` example `verify_replay`, or an
  `engine-cli` subcommand. It runs `Replay::from_json(...)?.verify()` and fails if the
  re-simulated `final_hash` differs from the recorded one. That is the "Python `final_hash`
  equals Rust `Replay::verify`" criterion. It could reuse `engine_wasm::check_replay`, which
  the parity check already uses. The episode could also be added to the parity fixtures once
  its format is settled.

Expected runtime, to confirm when M3 exists: the wheel job is dominated by the release build
(about 1 to 2 minutes cold, less with `rust-cache`). The test jobs take tens of seconds, plus
pip installs.

Failure semantics: any failing step fails the workflow. A failed parity step means Python
and Rust disagree about the same actions, which is a bridge bug (for example a lossy action
conversion), not flakiness. The job is separate, so `lint`, `test`, `wasm` and `nightly` are
unaffected whatever it does.
