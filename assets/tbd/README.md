# TBD vehicle model (physics sim, estimated parameters)

Internal, Coastal Agentics. **Physics sim with estimated parameters, not measured.** TBD is a working name.

- `tbd.xml` + `meshes/`: the MuJoCo model (MuJoCo 4.0.1). Same file for the browser (MuJoCo WASM) and native runs.
- `PARAMS.md`: every parameter, with source or an **est.** mark.
- `reference/`: the Python/JS prototype this was built from (controller, generator, validation). Reference only; the game logic is ported to `games/tbd`.
