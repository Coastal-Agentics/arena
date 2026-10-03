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
  3. the sim changed without regenerating. That's legitimate only with a bump of the format Tank Arena writes
     or a deliberate rule change, stated in the PR.

  The `test` job's `parity` test fails in cases 2 and 3 as well, so a red `wasm` job with a
  green `test` job points at case 1.
- **Also consider (optional):** `node scripts/check-viewer.mjs` (the viewer's pure helpers
  against `web/pkg`, about 0.2 s) isn't in CI today. It fits right after the parity step.

## (b) engine-py wheel build (M4: ready to apply)

`engine-py/` landed in M4 ([python.md](python.md)). This is the workflow for it, for the
workflow owner to add as `.github/workflows/engine-py.yml`. It is separate, so engine CI
isn't slowed, and `ci.yml` needs no change.

**Existing jobs are unaffected.** `engine-py` is its own Cargo workspace. The root
`Cargo.toml` lists it in `exclude`, and it has its own `Cargo.lock` and `target/`. Every
`cargo … --workspace` job (lint, test, wasm, nightly) never compiles pyo3 or needs Python.
The root `Cargo.lock` has no pyo3 entries. pyo3 is behind the crate's `python` feature,
which maturin turns on. maturin (1.9.4 or later) sets `PYO3_BUILD_EXTENSION_MODULE`
itself, so pyo3's deprecated `extension-module` feature isn't used. The wheel uses pyo3's
`abi3-py310`, so one wheel covers CPython 3.10 to 3.14.

**The workflow:** one wheel job (Rust checks, build, test on 3.10, native replay check) and
one job testing the same wheel on 3.14.

```yaml
name: engine-py

on:
  pull_request:
    paths: ["engine/**", "games/**", "engine-py/**", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/engine-py.yml"]
  push:
    branches: [main]
    paths: ["engine/**", "games/**", "engine-py/**", "Cargo.lock", "rust-toolchain.toml", ".github/workflows/engine-py.yml"]
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: engine-py-${{ github.ref }}
  cancel-in-progress: true

jobs:
  wheel:
    name: wheel (abi3-py310)
    # Pinned like test/wasm: the determinism pins are same-platform results (ADR-003).
    runs-on: ubuntu-24.04
    defaults:
      run:
        working-directory: engine-py
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
      - name: fmt
        run: cargo fmt --check
      - name: clippy (core, then with pyo3)
        run: |
          cargo clippy --all-targets -- -D warnings
          cargo clippy --all-targets --features python -- -D warnings
      - name: Rust tests (session vs native, pinned hashes, allocations, lockfile)
        run: cargo test --release
      - name: doc
        run: RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features python
      - name: Build one abi3 wheel
        run: |
          python -m pip install --disable-pip-version-check "maturin>=1.9.4,<2"
          maturin build --release --out dist
          ls -l dist
      - name: Test the bare wheel, numpy only (Python 3.10)
        run: |
          python -m venv "$RUNNER_TEMP/bare"
          "$RUNNER_TEMP/bare/bin/python" -m pip install --disable-pip-version-check dist/*.whl -r requirements-test.txt
          SALTMARSH_ARENA_EXTRAS=none "$RUNNER_TEMP/bare/bin/python" -m pytest -q
      - name: Test the wheel with [all] (Python 3.10)
        run: |
          python -m pip install --disable-pip-version-check "$(ls dist/*.whl)[all]" -r requirements-test.txt -r requirements-extras.txt
          SALTMARSH_ARENA_EXTRAS=all SALTMARSH_ARENA_REPLAY_DIR=target/py-replays python -m pytest -q
      - name: Verify the Python-run replays natively
        run: cargo run --release -q --example verify_replay -- target/py-replays/*.json
      - uses: actions/upload-artifact@v7
        with:
          name: saltmarsh-arena-wheel
          path: engine-py/dist/*.whl

  test:
    name: test (py3.14, same wheel)
    needs: wheel
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-python@v7
        with:
          python-version: "3.14"
      - uses: actions/download-artifact@v8
        with:
          name: saltmarsh-arena-wheel
          path: dist
      - name: Test the bare wheel, numpy only (Python 3.14)
        working-directory: engine-py
        run: |
          python -m venv "$RUNNER_TEMP/bare"
          "$RUNNER_TEMP/bare/bin/python" -m pip install --disable-pip-version-check ../dist/*.whl -r requirements-test.txt
          SALTMARSH_ARENA_EXTRAS=none "$RUNNER_TEMP/bare/bin/python" -m pytest -q
      - name: Test the wheel with [all] (Python 3.14)
        working-directory: engine-py
        run: |
          python -m pip install --disable-pip-version-check "$(ls ../dist/*.whl)[all]" -r requirements-test.txt -r requirements-extras.txt
          SALTMARSH_ARENA_EXTRAS=all python -m pytest -q
```

What the steps run (all in `engine-py/`):

- `cargo test --release`: `tests/session.rs` (a session's replay equals the native loop's,
  byte for byte, for both games), `tests/fixture.rs` (the pinned native final hashes in
  `tests/fixtures/determinism.json`), `tests/alloc.rs` (steps allocate only for history
  doubling) and `tests/lock.rs` (shared crates have the root lockfile's versions).
- `pytest`: `python/tests/`, twice per Python.
  - **Bare wheel (numpy only), `SALTMARSH_ARENA_EXTRAS=none`:** the API, `FlatEnv` and
    the Python-vs-native determinism episodes (including 1,000-step tank and racing
    episodes). It asserts that Gymnasium and PettingZoo are absent, and that the envs
    raise `MissingExtraError` naming `[gym]` or `[pettingzoo]`. The env tests skip.
  - **`[all]`, `SALTMARSH_ARENA_EXTRAS=all`:** everything, including PettingZoo's
    `parallel_api_test` and `parallel_seed_test`, Gymnasium's `check_env`, and the env
    determinism runs. A missing extra fails here instead of skipping.
  - The pins for the extras are in `requirements-extras.txt`, and pytest's pin is in
    `requirements-test.txt`.
- `verify_replay`: re-verifies, with no Python, every replay the Python determinism test
  wrote (`Replay::verify`). This is the "Python `final_hash` equals Rust's
  `Replay::verify`" criterion.

Measured locally (2026-10-03): the release build of the wheel takes about 11 s warm. The
Rust tests take under 1 s, and pytest takes about 1 s on each Python. A cold CI run is
dominated by compiling the engine crates and pyo3, about 1 to 2 minutes.

Failure semantics: any failing step fails the workflow. A failed determinism test or
`verify_replay` means Python and Rust disagree about the same actions. That is a bridge bug
(for example a lossy action conversion), not flakiness. The jobs are separate, so `lint`,
`test`, `wasm` and `nightly` are unaffected whatever they do. The workflow publishes
nothing: the wheel is a build artifact only, and publishing is a separate gate.
