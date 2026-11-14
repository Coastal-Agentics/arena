# Third-party notices

TBD is a physics sim with estimated parameters. TBD is the working name.
The Arena engine and the games in this repository are MIT, as in `LICENSE`.
The pieces below are used by the TBD sim and keep their own licenses.

## MuJoCo

Physics engine. Copyright Google DeepMind. Licensed under the Apache License, Version 2.0.

- Model file: `assets/tbd/tbd.xml`, authored for MuJoCo 4.0.1.
- Browser: the official WASM build published as npm `@mujoco/mujoco` 4.0.1. The page that loads it lives under `web/tbd/` and is not part of the crate in `games/tbd`.
- Native runner: the MuJoCo C library linked by `mujoco-rs` 7.1 (MuJoCo 3.17) when `tbd-native` is built with `--features native`.

https://mujoco.org — https://github.com/google-deepmind/mujoco

## mujoco-rs

Rust bindings used only by the optional `tbd-native` feature. Copyright David Hožič. Dual licensed under the MIT license or the Apache License, Version 2.0, with an additional reservation of rights for text and data mining and machine-learning training in the upstream `LICENSE`.

https://github.com/davidhozic/mujoco-rs — crates.io `mujoco-rs` 7.1.0

## three.js

JavaScript 3D renderer for the browser page. Copyright © 2010-2026 three.js authors. Licensed under the MIT license.

https://github.com/mrdoob/three.js

The page that vendors it is `web/tbd/` (not built by the `games/tbd` crate).
