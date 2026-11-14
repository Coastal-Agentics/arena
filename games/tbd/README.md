# games/tbd

TBD is a physics sim with estimated parameters. TBD is the working name.

This crate is the game step. It does not link MuJoCo. The browser page (owned separately, under `web/tbd/`) runs official MuJoCo WASM (`@mujoco/mujoco` 4.0.1) and calls `WasmTbd` in `web/pkg` once per physics tick. A native headless run is `tbd-native` with `--features native` (`tbd-native/README.md`).

The cascade, battery and mixer match `assets/tbd/reference/tbd_control.py` and `tbd_control.js`. The stair-hop script matches `validate.py`. Numbers are estimates (`assets/tbd/PARAMS.md`); nothing here was measured on hardware.

## JS API (`WasmTbd`)

After `await init()` from `web/pkg/engine_wasm.js`:

```js
import init, { WasmTbd, tbdObsLen, tbdCtrlLen } from "./pkg/engine_wasm.js";

const game = new WasmTbd(1n, JSON.stringify({
  dt: 0.002,            // seconds; use the model's timestep
  mission: "stair_hop", // or "manual"
  wind: true,
  max_time_s: 16
}));

// once per mj_step, after sensors are valid (mj_forward after a reset):
const obs = new Float64Array(tbdObsLen());
// fill obs — layout below
const ctrl = game.step(obs);          // Float64Array, length tbdCtrlLen() (9)
const state = JSON.parse(game.stateJson());
// copy ctrl into MjData.ctrl, in order
// state.wind_force_n is [fx, fy, fz] newtons, world frame.
// write it to xfrc_applied on body "tbd" (torque 0), then mj_step.

game.reset(); // same seed and config
game.free();
```

`new WasmTbd(seed, configJson)` throws if the config is not a JSON object, or if `dt` / `max_time_s` are not finite and positive. `""` and `"{}"` are the defaults (`stair_hop`, wind on, `dt` 0.002, 16 s). `seed` is a BigInt (`1n`). wasm-bindgen 0.2.100 does not accept a Number for this `u64`.

`step` throws if `obs.length` is not 20 or any value is NaN or infinite. After `state.over` is true, `step` returns the last command and does not move the clock or the wind stream.

`stateJson()` fields the page needs: `mode` (`ground` | `unfolding` | `flight` | `landing` | `folding`), `phase` (the stair-hop script name, or the mode name when `mission` is `manual`), `battery_pct`, `power_w`, `score`, `timer_s`, `completed`, `landed_on_landing`, `over`, `wind_world_mps`, `wind_force_n`, `ctrl`.

Wind is seeded from `seed` with the same ChaCha8 construction as the engine RNG. The same seed, config and observation log reproduce commands and `wind_force_n`. The controller does not cancel that force; the page applies it.

## Observation (20) and command (9)

Canonical names: `OBS_NAMES` and `CTRL_NAMES` in the crate (`obs` module). They follow `tbd.xml`.

| index | observation | unit | `tbd.xml` |
| --- | --- | --- | --- |
| 0..2 | gyro x, y, z | rad/s | sensor `gyro` (site `imu`) |
| 3..5 | accel x, y, z | m/s² | sensor `accel` |
| 6..9 | quat w, x, y, z | 1 | sensor `quat` (framequat) |
| 10 | range down | m | sensor `range_down` (`-1` on a miss) |
| 11 | fold angle | rad | sensor `fold_angle` (`fold_FL`) |
| 12..14 | position x, y, z | m | free joint `root` `qpos[0..3]` (body origin, not the imu site) |
| 15..17 | velocity x, y, z | m/s | free joint `root` `qvel[0..3]`, world frame |
| 18 | pilot_x | 1 | page stick, `manual` only |
| 19 | pilot_yaw | 1 | page stick, `manual` only |

| index | command | range | actuator |
| --- | --- | --- | --- |
| 0..3 | thrust FL, FR, RL, RR | 0 .. 1 | `thrust_FL` … `thrust_RR` |
| 4..7 | drive FL, FR, RL, RR | −26 .. 26 rad/s | `drive_FL` … `drive_RR` |
| 8 | fold | 0 .. 1.5708 rad | `fold` (0 deployed, 1.5708 stowed) |

On this model the imu site is aligned with the body, so the gyro matches the body rates the reference controller reads from `qvel[3:6]`, and the frame quaternion matches the free-joint quaternion.

`stair_hop` ignores the pilot channels. `manual` uses them inside ±1: `pilot_x` drives forward (+X when both wheel sides are positive), `pilot_yaw` skids (positive speeds the right side and slows the left). Sentinels, one tick, and that channel is not also a stick: `pilot_x >= 2` takeoff, `pilot_x <= -2` land, `pilot_yaw >= 2` fold once the vehicle is down.

## Score

Stair-hop points, from the crate docs on `stair_score`: partial credit `100 * clamp(x / 2.30, 0, 1)` until touchdown on the deck, then

`max(0, 1000 − 25·duration_s − 4·max_tilt_deg − 8·battery_pct_used + 200·clearance_m)`

with clearance clamped to `[0, 0.5]`. `manual` leaves the score at 0.
