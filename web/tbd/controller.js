// TBD sim controller: the JS reference implementation.
// Physics sim with ESTIMATED parameters; nothing here is measured. TBD is a working name.
//
// Step interface (shared with the planned Rust/wasm controller, see controller-wasm.js):
//   const ctl = createJsController();
//   ctl.reset(state)                  // after a physics reset
//   ctl.press(action)                 // one-shot actions: "fly" | "hop"
//   const cmd = ctl.step(state, input)
//
//   state (sensors in):  { dt, pos:[3] world m, quat:[w,x,y,z], vel:[3] world m/s, omega:[3] body rad/s,
//                          agl: m above ground (range sensor), fold: fold angle rad, battery: % left }
//   input (pilot in):    { fwd, turn, strafe, climb, yaw }  each in [-1, 1]
//   cmd (commands out):  { mode, auto, drive:[left, right] rad/s, thrust:[4] in [0,1] (FL FR RL RR),
//                          force:[4] N, fold: target fold angle rad, driving: 0..1 }
//
// The controller never touches MuJoCo; main.js copies cmd into data.ctrl and steps physics.
// Ported 1:1 from the box prototype's flight controller and battery model.

export const G = 9.81;
export const P = {
  mass: 0.970, t_max: 5.50, k_yaw: 0.012, x_arm: 0.055, y_arm: 0.1115,
  I: [0.0046, 0.0040, 0.0075],
  att_wn: 14.0, att_zeta: 0.9, yaw_kp: 16.0, yaw_kd: 8.0,
  pos_kp: 5.0, pos_kd: 4.0, alt_kp: 8.0, alt_kd: 5.0, alt_ki: 2.0,
  max_tilt: 0.40, max_acc: 4.0,
  g_per_w: 2.3, avionics_w: 6.0, pack_wh: 18.9, usable: 0.80, wheel_w: 1.0,
};
export const W_MAX = 26.0;   // wheel speed limit, rad/s
export const STOW = 1.5708;  // fold angle when stowed (ground), rad; 0 = deployed (flight)
const SPIN = [1, -1, -1, 1];                       // FL FR RL RR
const XS = [1, 1, -1, -1].map(s => s * P.x_arm);
const YS = [1, -1, 1, -1].map(s => s * P.y_arm);
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
export const wrap = a => ((a + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;

export function quatToRpy(w, x, y, z) {
  return [Math.atan2(2 * (w * x + y * z), 1 - 2 * (x * x + y * y)),
          Math.asin(clamp(2 * (w * y - z * x), -1, 1)),
          Math.atan2(2 * (w * z + x * y), 1 - 2 * (y * y + z * z))];
}
export function mix(F, tx, ty, tz) {
  return [0, 1, 2, 3].map(i => clamp(F / 4 + tx / (4 * YS[i]) - ty / (4 * XS[i]) + tz / (4 * SPIN[i] * P.k_yaw), 0, P.t_max));
}

// Cascaded position -> attitude -> thrust controller (assisted hover).
export class FlightController {
  constructor() { this.iz = 0; }
  reset() { this.iz = 0; }
  step(dt, pos, quat, vel, omega, pDes, vDes = [0, 0, 0], yawDes = 0) {
    const [roll, pitch, yaw] = quatToRpy(...quat);
    const e = [0, 1, 2].map(i => pDes[i] - pos[i]), ev = [0, 1, 2].map(i => vDes[i] - vel[i]);
    let ax = P.pos_kp * e[0] + P.pos_kd * ev[0], ay = P.pos_kp * e[1] + P.pos_kd * ev[1];
    const n = Math.hypot(ax, ay); if (n > P.max_acc) { ax *= P.max_acc / n; ay *= P.max_acc / n; }
    const cy = Math.cos(yaw), sy = Math.sin(yaw);
    const axb = cy * ax + sy * ay, ayb = -sy * ax + cy * ay;
    const pitchD = clamp(Math.atan2(axb, G), -P.max_tilt, P.max_tilt);
    const rollD = clamp(-Math.atan2(ayb, G), -P.max_tilt, P.max_tilt);
    this.iz = clamp(this.iz + e[2] * dt, -2, 2);
    const az = P.alt_kp * e[2] + P.alt_kd * ev[2] + P.alt_ki * this.iz;
    const F = P.mass * (G + az) / Math.max(0.5, Math.cos(roll) * Math.cos(pitch));
    const wn = P.att_wn, z = P.att_zeta, [Ix, Iy, Iz] = P.I;
    const tx = Ix * (wn * wn * (rollD - roll) - 2 * z * wn * omega[0]);
    const ty = Iy * (wn * wn * (pitchD - pitch) - 2 * z * wn * omega[1]);
    const tz = Iz * (P.yaw_kp * wrap(yawDes - yaw) - P.yaw_kd * omega[2]);
    const f = mix(F, tx, ty, tz);
    return { u: f.map(v => v / P.t_max), f, rpy: [roll, pitch, yaw] };
  }
}

// Estimated battery model: hover power from a grams-per-watt figure, plus avionics and wheels.
export class Battery {
  constructor() { this.whUsed = 0; this.lastW = 0; this.th = P.mass * G / 4; this.ph = (this.th / G * 1000) / P.g_per_w; }
  power(f, driving = 0) { return f.reduce((s, t) => s + this.ph * Math.pow(Math.max(t, 0) / this.th, 1.5), 0) + P.avionics_w + driving * 4 * P.wheel_w; }
  step(dt, f, driving = 0) { this.lastW = this.power(f, driving); this.whUsed += this.lastW * dt / 3600; return this.pct(); }
  pct() { return Math.max(0, 100 * (1 - this.whUsed / (P.pack_wh * P.usable))); }
  minutesLeft() { return this.lastW > 1 ? (P.pack_wh * P.usable - this.whUsed) / this.lastW * 60 : 0; }
  hoverMinutes() { return P.pack_wh * P.usable / (4 * this.ph + P.avionics_w) * 60; }
}

// ---------------------------------------------------------------- input mapping
// Held keys -> analog axes. Each entry is [axis, sign].
export const KEY_AXES = {
  KeyW: [['fwd', 1]], ArrowUp: [['fwd', 1]], KeyS: [['fwd', -1]], ArrowDown: [['fwd', -1]],
  KeyD: [['turn', 1], ['strafe', 1]], ArrowRight: [['turn', 1], ['strafe', 1]],
  KeyA: [['turn', -1], ['strafe', -1]], ArrowLeft: [['turn', -1], ['strafe', -1]],
  Space: [['climb', 1]], ShiftLeft: [['climb', -1]], ShiftRight: [['climb', -1]],
  KeyQ: [['yaw', 1]], KeyE: [['yaw', -1]],
};
// Key presses -> one-shot actions.
export const KEY_ACTIONS = { KeyF: 'fly', KeyG: 'hop', KeyR: 'reset', KeyC: 'camera', KeyP: 'pause', KeyH: 'help' };
// Gamepad buttons (standard mapping) -> one-shot actions.
export const PAD_ACTIONS = { 0: 'fly', 1: 'reset', 3: 'camera', 2: 'hop' };
export const AXES = ['fwd', 'turn', 'strafe', 'climb', 'yaw'];
export const deadzone = (v, dz = 0.12) => (Math.abs(v) < dz ? 0 : v);

// keys: Set of KeyboardEvent.code; pad: {lx, ly, rx, ry, lt, rt} or null; touch: {x, y, climb} or null.
export function readInput(keys, pad = null, touch = null) {
  const input = { fwd: 0, turn: 0, strafe: 0, climb: 0, yaw: 0 };
  for (const code of keys) for (const [axis, sign] of KEY_AXES[code] || []) input[axis] += sign;
  if (pad) {
    input.fwd -= pad.ly; input.turn += pad.lx; input.strafe += pad.lx;
    input.climb += -pad.ry + (pad.rt || 0) - (pad.lt || 0); input.yaw -= pad.rx;
  }
  if (touch) {   // left joystick: up = forward, sideways = turn (skid steer on the ground, yaw in flight)
    input.fwd -= touch.y || 0; input.turn += touch.x || 0; input.yaw -= touch.x || 0;
    input.climb += touch.climb || 0;   // Climb / Descend buttons: +1 / -1
  }
  for (const a of AXES) input[a] = clamp(input[a], -1, 1);
  return input;
}

// ---------------------------------------------------------------- mode machine + autopilot
export const MODES = ['ground', 'deploying', 'flight', 'landing', 'stowing'];
const leash = (t, p, L = 0.6) => t.map((v, i) => p[i] + clamp(v - p[i], -L, L));

export function createJsController() {
  const fc = new FlightController();
  let mode = 'ground', target = [0, 0, 0], yawT = 0, landTimer = 0, fold = STOW, auto = null, t = 0;
  let pending = [];
  const out = (drive, thrust, force, driving) => ({ mode, auto: auto && auto.phase, drive, thrust, force, fold, driving });
  const zero4 = () => [0, 0, 0, 0];
  const drive = (l, r) => [clamp(l, -W_MAX, W_MAX), clamp(r, -W_MAX, W_MAX)];

  function autoStep(s) {
    const p = s.pos, v = s.vel, tp = t - auto.t0, set = ph => { auto.phase = ph; auto.t0 = t; };
    const TOP = 0.68, zc = TOP + 0.32, xland = 2.30;
    let vd = [0, 0, 0];
    switch (auto.phase) {
      case 'drive': mode = 'ground'; if (p[0] >= 0.42) { set('brake'); return out([0, 0], zero4(), zero4(), 0); } return out(drive(24, 24), zero4(), zero4(), 1);
      case 'brake': if (tp > 0.5) { fold = 0; mode = 'deploying'; set('deploy'); } return out([0, 0], zero4(), zero4(), 0);
      case 'deploy': if (s.fold < 0.04 && tp > 0.4) { target = p.slice(); fc.reset(); mode = 'flight'; set('climb'); } return out([0, 0], zero4(), zero4(), 0);
      case 'climb': target[2] = Math.min(zc, target[2] + 0.9 * s.dt); vd = [0, 0, target[2] < zc ? 0.9 : 0]; if (p[2] > zc - 0.06) set('transit'); break;
      case 'transit': target[0] = Math.min(xland, target[0] + 0.7 * s.dt); vd = [target[0] < xland ? 0.7 : 0, 0, 0];
        if (Math.abs(p[0] - xland) < 0.03 && Math.hypot(...v) < 0.1) { mode = 'landing'; set('descend'); } break;
      case 'descend': target[2] = Math.max(TOP - 0.05, target[2] - 0.35 * s.dt); vd = [0, 0, -0.35]; if (p[2] < TOP + 0.004 && tp > 0.5) set('touchdown'); break;
      case 'touchdown': if (tp > 0.4) { fold = STOW; mode = 'stowing'; set('stow'); } return out([0, 0], zero4(), zero4(), 0);
      case 'stow': if (tp > 0.9) { mode = 'ground'; set('roll'); } return out([0, 0], zero4(), zero4(), 0);
      case 'roll': if (tp > 1.0) { auto = null; return out([0, 0], zero4(), zero4(), 0); } return out(drive(24, 24), zero4(), zero4(), 1);
    }
    const r = fc.step(s.dt, p, s.quat, v, s.omega, target, vd, 0);
    return out([0, 0], r.u, r.f, 0);
  }

  function toggleFly() {
    if (auto) return;
    if (mode === 'ground') { mode = 'deploying'; fold = 0; }
    else if (mode === 'flight') { mode = 'landing'; landTimer = 0; }
    else if (mode === 'landing') mode = 'flight';
  }

  return {
    name: 'js',
    get mode() { return mode; },
    get auto() { return auto && auto.phase; },
    reset(s, startMode = 'ground') {
      mode = startMode; target = s.pos.slice(); yawT = 0; landTimer = 0; auto = null; t = 0; pending = [];
      fold = startMode === 'ground' ? STOW : 0; fc.reset();
    },
    startAuto(s) { this.reset(s); auto = { phase: 'drive', t0: 0 }; },
    press(action) { if (action === 'fly') toggleFly(); },
    step(s, input) {
      let cmd;
      if (auto) cmd = autoStep(s);
      else if (mode === 'ground' || mode === 'stowing') {
        const { fwd, turn } = input;
        if (mode === 'stowing' && s.fold > STOW - 0.06) mode = 'ground';
        cmd = out(drive(W_MAX * (fwd + turn), W_MAX * (fwd - turn)), zero4(), zero4(), Math.min(1, Math.abs(fwd) + Math.abs(turn)));
      } else if (mode === 'deploying') {
        if (s.fold < 0.05) { mode = 'flight'; fc.reset(); target = s.pos.slice(); target[2] += 0.30; yawT = quatToRpy(...s.quat)[2]; }
        cmd = out([0, 0], zero4(), zero4(), 0);
      } else {   // flight / landing: assisted hover (velocity commands, position hold when sticks are centred)
        const yaw = quatToRpy(...s.quat)[2], c = Math.cos(yaw), sn = Math.sin(yaw);
        const vb = [input.fwd * 1.2, -input.strafe * 1.2];
        const vd = [c * vb[0] - sn * vb[1], sn * vb[0] + c * vb[1], input.climb * 0.8];
        yawT += input.yaw * 1.6 * s.dt;
        if (mode === 'landing') {
          vd[2] = -0.4;
          if (s.agl < 0.02 && Math.abs(s.vel[2]) < 0.15) landTimer += s.dt; else landTimer = 0;
          if (landTimer > 0.25) { mode = 'stowing'; fold = STOW; }
        }
        if (s.battery <= 0 && mode === 'flight') mode = 'landing';
        target = leash(target.map((v, i) => v + vd[i] * s.dt), s.pos);
        if (mode === 'flight') target[2] = Math.max(target[2], s.pos[2] - s.agl + 0.05);
        if (mode === 'flight' || mode === 'landing') {
          const r = fc.step(s.dt, s.pos, s.quat, s.vel, s.omega, target, vd, yawT);
          cmd = out([0, 0], r.u, r.f, 0);
        } else cmd = out([0, 0], zero4(), zero4(), 0);
      }
      t += s.dt;
      return cmd;
    },
  };
}
