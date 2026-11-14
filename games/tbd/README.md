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
  max_time_s: 16,
  voltage_sag: true,    // all four default on
  ram_drag: true,
  ground_effect: true,
  fold_rate_limit: true,
  r_cell_ohm: 0.0075    // 7.5 mΩ per cell
}));

// once per mj_step, after sensors are valid (mj_forward after a reset):
const obs = new Float64Array(tbdObsLen());
// fill obs — layout below
const ctrl = game.step(obs);          // Float64Array, length tbdCtrlLen() (9)
const state = JSON.parse(game.stateJson());
// copy ctrl into MjData.ctrl, in order
// xfrc_applied on body "tbd", world frame, before mj_step:
//   force  = state.wind_force_n + state.aero_force_n
//   torque = state.aero_torque_nm   (about the body centre of mass)

game.reset(); // same seed and config
game.free();
```

`new WasmTbd(seed, configJson)` throws if the config is not a JSON object, or if `dt` / `max_time_s` are not finite and positive. `""` and `"{}"` are the defaults (`stair_hop`, wind on, `dt` 0.002, 16 s). `seed` is a BigInt (`1n`). wasm-bindgen 0.2.100 does not accept a Number for this `u64`.

`step` throws if `obs.length` is not 20 or any value is NaN or infinite. After `state.over` is true, `step` returns the last command and does not move the clock or the wind stream.

`stateJson()` fields the page needs: `mode` (`ground` | `unfolding` | `flight` | `landing` | `folding`), `phase` (the stair-hop script name, or the mode name when `mission` is `manual`), `battery_pct`, `power_w`, `score`, `timer_s`, `completed`, `landed_on_landing`, `over`, `wind_world_mps`, `wind_force_n`, `aero_force_n`, `aero_torque_nm`, `pack_voltage_v`, `pack_current_a`, `ground_effect`, `ctrl`.

## Estimated physics switches

All four default **on**. Set the boolean to `false` to drop that effect. Sources and the ESTIMATE marks are in `assets/tbd/PARAMS.md`.

| Field | Default | What it does |
| --- | --- | --- |
| `voltage_sag` | `true` | `V = V_oc − I R`. `V_oc` runs linearly from 4.2 V to 3.5 V per cell across 6S. Thrust scales with `(V / 25.2 V)²`. Hover draw is about 19 A. |
| `r_cell_ohm` | `0.0075` | Per-cell resistance, ohms (7.5 mΩ). Band 5–10 mΩ. **ESTIMATE, unsourced; fit from a logged flight or measure with a charger IR readout.** |
| `ram_drag` | `true` | Per duct, `F ≈ ṁ V_air` opposite the in-plane airspeed. `ṁ = T / v_i`, `v_i = √(T / (2 ρ A))`, `A = π (0.038)²`. About 2–3 N total at 4 m/s, plus a small pitch moment because the ducts sit below the body centre of mass. Reported as `aero_force_n` and `aero_torque_nm`. |
| `ground_effect` | `true` | `T_IGE / T_OGE = 1 / (1 − (R / 4h)²)`, `R = 0.038` m. `h` comes from `range_down` plus 23 mm (duct plane 37 mm, rangefinder site 14 mm), clamped to at least `R/2`. About +7% at liftoff and about 1 at 1 m. Cheeseman and Bennett, open-rotor formula, so an estimate for a duct. The ratio is also `state.ground_effect`. |
| `fold_rate_limit` | `true` | Fold command slews at about 465°/s, so 90° takes about 0.19 s. ES08MD II no-load figure (0.10 s per 60° at 6 V, ±58° of horn travel). The XML `kp` is unchanged; a position actuator has no rate attribute. |

Kept as they were: motor lag τ = 40 ms, 5.5 N per duct as an optimistic placeholder (±10–20%, open prop not the ducted D76), hover efficiency 2.3 g/W as a placeholder with range 1.8–2.5 (**ESTIMATE**).

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
