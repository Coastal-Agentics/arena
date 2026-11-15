# tbd-native

Headless runner for TBD, a physics sim with estimated parameters. TBD is the working name.

The binary loads `assets/tbd/tbd.xml`, steps MuJoCo, and calls `games/tbd` once per physics tick. It writes actuator commands into `ctrl` and the crate's estimated wind force into `xfrc_applied` on body `tbd`. A summary goes to stdout. `--out` writes a trace (50 Hz plus phase changes, or every tick with `--full`).

```sh
cargo run -p tbd-native -- --seed 1
```

That build has no MuJoCo in it. It exits 2 and prints how to turn the feature on. `cargo test --workspace` and `cargo clippy --workspace --all-targets` do the same: they compile this crate without the `native` feature, so they do not need the MuJoCo C library.

## Run with MuJoCo

`mujoco-rs` 7.1 links **MuJoCo 3.17**. The model file and the browser build are **MuJoCo 4.0.1**. The XML is the same file. Last-bit differences between those builds are expected; this runner does not claim bit-exact parity with the browser.

The `native` feature turns on `mujoco-rs`'s `auto-download-mujoco`, which downloads the shared library at build time. `MUJOCO_DOWNLOAD_DIR` must be an absolute path.

```sh
export MUJOCO_DOWNLOAD_DIR="$HOME/.cache/mujoco-3.17"
mkdir -p "$MUJOCO_DOWNLOAD_DIR"
cargo run -p tbd-native --features native -- --seed 1 --out /tmp/tbd-trace.json
```

The download is a shared library. `auto-download` unpacks an archive under `MUJOCO_DOWNLOAD_DIR`, and the linker needs the directory that contains `libmujoco.so` (for 7.1 that is `mujoco-3.17.0/lib` inside the download directory):

```sh
export LD_LIBRARY_PATH="$MUJOCO_DOWNLOAD_DIR/mujoco-3.17.0/lib:${LD_LIBRARY_PATH:-}"
```

Other switches: `--no-wind`, `--max-time 4`, `--scenario manual`, `--xml path/to/tbd.xml`, `--full`.

## Optional test

Not part of default CI (CI does not pass `--features native`):

```sh
cargo test -p tbd-native --features native
```

That loads the model, steps 0.2 s of the stair-hop twice, and checks the summaries match.
