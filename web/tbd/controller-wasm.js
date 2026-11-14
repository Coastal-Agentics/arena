// Adapter for the Rust/wasm TBD controller (games/tbd, WasmTbd in web/pkg, PR #132).
// Physics sim with ESTIMATED parameters. TBD is a working name.
//
// It wraps WasmTbd in the same interface as createJsController() in controller.js:
//   { name, mode, auto, reset(state, startMode), startAuto(state), press(action), step(state, input) -> cmd }
// so main.js does not care which one runs. The JS controller stays the test reference and the fallback:
// if ../pkg/engine_wasm.js is missing, fails to load, or has no WasmTbd, pickController() returns JS.
//
// WasmTbd contract (games/tbd/README.md):
//   obs[20]  = sensordata[0..12] (gyro 3, accel 3, quat wxyz 4, range_down, fold_angle),
//              root qpos xyz, root qvel xyz, pilot_x, pilot_yaw
//   step(obs) -> 9 commands in <actuator> order; call once per mj_step.
//   stateJson().wind_force_n -> world-frame force for xfrc_applied on body "tbd".
//   manual mission: pilot_x / pilot_yaw sticks in [-1, 1]; sentinels pilot_x >= 2 take off,
//   pilot_x <= -2 land, pilot_yaw >= 2 fold once down.

export const PILOT_TAKEOFF = 2, PILOT_LAND = -2, PILOT_FOLD = 2;
// Engine mode names -> the JS controller's names (labels, eye colour, camera).
export const MODE_FROM_ENGINE = { ground: 'ground', unfolding: 'deploying', flight: 'flight', landing: 'landing', folding: 'stowing' };
const clamp1 = v => Math.max(-1, Math.min(1, v));

// Pilot channels for manual mode from the shared input axes.
// Ground: pilot_yaw > 0 speeds the right side (turns left), so D / joystick right (turn +1) is pilot_yaw -1.
// Flight: pilot_yaw > 0 yaws left (Q). The engine has no strafe or climb channel, so A/D also yaw.
export function pilotFromInput(mode, input) {
  if (mode === 'flight') return [clamp1(input.fwd), clamp1(input.yaw !== 0 ? input.yaw : -input.turn)];
  return [clamp1(input.fwd), clamp1(-input.turn)];
}

// Build the 20-value observation. sensors: sensordata[0..12]; pilot: [x, yaw].
export function buildObs(obs, state, pilot) {
  for (let i = 0; i < 12; i++) obs[i] = state.sensors[i];
  obs[12] = state.pos[0]; obs[13] = state.pos[1]; obs[14] = state.pos[2];
  obs[15] = state.vel[0]; obs[16] = state.vel[1]; obs[17] = state.vel[2];
  obs[18] = pilot[0]; obs[19] = pilot[1];
  return obs;
}

// Wraps a loaded engine_wasm module. opts: { seed (BigInt-able), wind (bool), dt, maxTime }.
export function createWasmController(mod, opts = {}) {
  const seed = BigInt(opts.seed ?? 1), wind = opts.wind ?? true, dt = opts.dt ?? 0.002;
  const make = mission => new mod.WasmTbd(seed, JSON.stringify({ dt, mission, wind, max_time_s: mission === 'stair_hop' ? (opts.maxTime ?? 16) : 1e9 }));
  const obs = new Float64Array(mod.tbdObsLen());
  let game = make('manual'), mission = 'manual', st = JSON.parse(game.stateJson()), sentinel = null, hopResult = null;
  const swap = m => { game.free(); game = make(m); mission = m; st = JSON.parse(game.stateJson()); sentinel = null; };
  return {
    name: 'wasm',
    get mode() { return MODE_FROM_ENGINE[st.mode] || st.mode; },
    get auto() { return mission === 'stair_hop' && !st.over ? st.phase : null; },
    get state() { return st; },
    get hopResult() { return hopResult; },
    reset(_s) { swap('manual'); },
    startAuto(_s) { hopResult = null; swap('stair_hop'); },
    press(action) {
      if (action !== 'fly' || mission !== 'manual') return;
      if (st.mode === 'ground') sentinel = [PILOT_TAKEOFF, 0];
      else if (st.mode === 'flight') sentinel = [PILOT_LAND, 0];
    },
    step(s, input) {
      let pilot = [0, 0];
      if (mission === 'manual') {
        if (sentinel) { pilot = sentinel; sentinel = null; }
        else if (st.mode === 'landing') pilot = [0, PILOT_FOLD];   // fold as soon as the engine sees it is down (as JS does)
        else pilot = pilotFromInput(st.mode, input);
      }
      const ctrl = Array.from(game.step(buildObs(obs, s, pilot)));
      st = JSON.parse(game.stateJson());
      const cmd = { mode: MODE_FROM_ENGINE[st.mode] || st.mode, auto: this.auto, ctrl, wind: st.wind_force_n,
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

// Resolves to { mod } when ../pkg/engine_wasm.js loads and exports WasmTbd, else null.
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

// Picks the controller for the page. The wasm controller is opt-in (?ctrl=wasm) for now;
// anything else, or any failure, gives the JS reference controller.
export async function pickController(prefer, createJs, opts = {}) {
  if (prefer === 'wasm') {
    try { const c = await loadWasmController(opts); if (c) return c; } catch (err) { console.warn('[tbd] wasm controller failed, using JS', err); }
  }
  return createJs();
}
