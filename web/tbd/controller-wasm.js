// Adapter for the Rust/wasm TBD controller (games/tbd, WasmTbd in web/pkg; PRs #132 and #133).
// Physics sim with ESTIMATED parameters. TBD is a working name.
//
// It wraps WasmTbd in the same interface as createJsController() in controller.js:
//   { name, mode, auto, reset(state, startMode), startAuto(state), press(action), step(state, input) -> cmd }
// so main.js does not care which one runs. The JS controller stays the test reference and the fallback.
//
// WasmTbd contract (games/tbd/README.md):
//   obs[tbdObsLen()] = sensordata[0..12] (gyro 3, accel 3, quat wxyz 4, range_down, fold_angle),
//                      root qpos xyz, root qvel xyz, pilot_x, pilot_yaw, [pilot_y, pilot_z]   (22 since #133, 20 in #132)
//   step(obs) -> 9 commands in <actuator> order; call once per mj_step.
//   xfrc_applied on body "tbd" (world frame): force = wind_force_n + aero_force_n, torque = aero_torque_nm
//   (#132 has no aero fields; they read as 0).
//   manual sentinels: pilot_x >= 2 take off (or back to flight while landing, #133), pilot_x <= -2 land,
//   pilot_yaw >= 2 stow once down (#133: holding it through touchdown is enough).

export const PILOT_TAKEOFF = 2, PILOT_LAND = -2, PILOT_FOLD = 2;
export const OBS_LEN_MANUAL = 22;   // first layout with strafe and climb channels (#133)
// #133 reports the JS mode names. #132 used unfolding/folding; alias those so a #132 build still labels right.
const MODE_ALIAS_132 = { unfolding: 'deploying', folding: 'stowing' };
export const modeName = m => MODE_ALIAS_132[m] || m;
const clamp1 = v => Math.max(-1, Math.min(1, v)) || 0;   // no -0 in the obs
const add3 = (a, b) => [0, 1, 2].map(i => (a?.[i] || 0) + (b?.[i] || 0));

// Pilot channels for manual mode from the shared input axes -> [pilot_x, pilot_yaw, pilot_y, pilot_z].
// Ground: pilot_yaw > 0 speeds the right side (turns left), so D / joystick right (turn +1) is pilot_yaw -1.
// Flight: pilot_yaw > 0 yaws left (Q/E, right stick, touch turn buttons); pilot_y > 0 strafes left, so
// D / stick right (strafe +1) is pilot_y -1; pilot_z is the climb rate (Space/Shift, Climb/Descend).
// A 20-slot build (#132) has no strafe or climb, so A/D yaw instead.
export function pilotFromInput(mode, input, obsLen = OBS_LEN_MANUAL) {
  if (mode !== 'flight') return [clamp1(input.fwd), clamp1(-input.turn), 0, 0];
  if (obsLen >= OBS_LEN_MANUAL) return [clamp1(input.fwd), clamp1(input.yaw), clamp1(-input.strafe), clamp1(input.climb)];
  return [clamp1(input.fwd), clamp1(input.yaw !== 0 ? input.yaw : -input.strafe), 0, 0];
}

// Fill the observation. sensors: sensordata[0..12]; pilot: [x, yaw, y, z] (extra slots dropped on a 20-slot build).
export function buildObs(obs, state, pilot) {
  for (let i = 0; i < 12; i++) obs[i] = state.sensors[i];
  obs[12] = state.pos[0]; obs[13] = state.pos[1]; obs[14] = state.pos[2];
  obs[15] = state.vel[0]; obs[16] = state.vel[1]; obs[17] = state.vel[2];
  for (let i = 18; i < obs.length; i++) obs[i] = pilot[i - 18] || 0;
  return obs;
}

// Wraps a loaded engine_wasm module. opts: { seed, wind (bool), dt, maxTime, physics (bool, #133 switches) }.
export function createWasmController(mod, opts = {}) {
  const obsLen = mod.tbdObsLen();
  const seed = BigInt(opts.seed ?? 1), wind = opts.wind ?? true, dt = opts.dt ?? 0.002;
  const config = mission => {
    const c = { dt, mission, wind, max_time_s: mission === 'stair_hop' ? (opts.maxTime ?? 16) : 1e9 };
    if (obsLen >= OBS_LEN_MANUAL && opts.physics === false) Object.assign(c, { voltage_sag: false, ram_drag: false, ground_effect: false, fold_rate_limit: false });
    return JSON.stringify(c);
  };
  const make = mission => new mod.WasmTbd(seed, config(mission));
  const obs = new Float64Array(obsLen);
  let game = make('manual'), mission = 'manual', st = JSON.parse(game.stateJson()), sentinel = null, hopResult = null;
  const swap = m => { game.free(); game = make(m); mission = m; st = JSON.parse(game.stateJson()); sentinel = null; };
  return {
    name: 'wasm',
    obsLen,
    hasManualControls: obsLen >= OBS_LEN_MANUAL,
    get mode() { return modeName(st.mode); },
    get auto() { return mission === 'stair_hop' && !st.over ? st.phase : null; },
    get state() { return st; },
    get hopResult() { return hopResult; },
    reset(_s) { swap('manual'); },
    startAuto(_s) { hopResult = null; swap('stair_hop'); },
    press(action) {
      if (action !== 'fly' || mission !== 'manual') return;
      const m = modeName(st.mode);
      if (m === 'ground') sentinel = [PILOT_TAKEOFF, 0, 0, 0];
      else if (m === 'flight') sentinel = [PILOT_LAND, 0, 0, 0];
      else if (m === 'landing' && obsLen >= OBS_LEN_MANUAL) sentinel = [PILOT_TAKEOFF, 0, 0, 0];   // back to flight (#133)
    },
    step(s, input) {
      let pilot = [0, 0, 0, 0];
      if (mission === 'manual') {
        const m = modeName(st.mode);
        if (sentinel) { pilot = sentinel; sentinel = null; }
        // Hold the stow sentinel while landing so it folds once down, as the JS controller does.
        // The engine never stows on its own (#132 or #133); #133 just takes it on the touchdown tick.
        else if (m === 'landing') pilot = [0, PILOT_FOLD, 0, 0];
        else pilot = pilotFromInput(m, input, obsLen);
      }
      const ctrl = Array.from(game.step(buildObs(obs, s, pilot)));
      st = JSON.parse(game.stateJson());
      const cmd = { mode: modeName(st.mode), auto: this.auto, ctrl, wind: add3(st.wind_force_n, st.aero_force_n), torque: add3(st.aero_torque_nm),
        thrust: ctrl.slice(0, 4), drive: [ctrl[4], ctrl[5]], fold: ctrl[8], force: ctrl.slice(0, 4).map(u => u * 5.5),
        driving: ctrl.slice(4, 8).some(v => v !== 0) ? 1 : 0, battery: st.battery_pct, powerW: st.power_w, engine: st };
      if (mission === 'stair_hop' && st.over) {   // hop done: keep the result, hand the sticks back
        hopResult = { score: st.score, time: st.time_s, completed: st.completed, landed: st.landed_on_landing,
          maxTiltDeg: st.max_tilt_deg, minClearance: st.min_clearance_m, batteryUsed: st.battery_pct_used };
        swap('manual');
      }
      return cmd;
    },
    free() { game.free(); },
  };
}

// Resolves to the engine_wasm module when it loads and exports WasmTbd, else null.
export async function loadWasmModule(url = new URL('../pkg/engine_wasm.js', import.meta.url).href) {
  try {
    const mod = await import(url);
    if (typeof mod.WasmTbd !== 'function' || typeof mod.tbdObsLen !== 'function') return null;
    await mod.default();
    return mod;
  } catch (err) {
    console.info('[tbd] wasm controller unavailable, using JS:', err && err.message);
    return null;
  }
}

export async function loadWasmController(opts = {}) {
  const mod = await loadWasmModule(opts.url);
  return mod ? createWasmController(mod, opts) : null;
}

// Picks the page's controller.
//   ?ctrl=js               -> JS reference controller.
//   default                -> wasm when ../pkg exports WasmTbd with the manual controls (tbdObsLen() >= 22), else JS.
//   ?ctrl=wasm             -> wasm whenever WasmTbd exists (also a 20-slot #132 build), else JS.
export async function pickController(prefer, createJs, opts = {}) {
  if (prefer === 'js') return createJs();
  try {
    const mod = await loadWasmModule(opts.url);
    if (mod && (prefer === 'wasm' || mod.tbdObsLen() >= OBS_LEN_MANUAL)) return createWasmController(mod, opts);
  } catch (err) { console.warn('[tbd] wasm controller failed, using JS', err); }
  return createJs();
}
