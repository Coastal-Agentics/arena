// TBD flight controller + battery model. 1:1 port of ../tbd_control.py (keep them in sync).
// Estimated parameters only (see ../PARAMS.md).
export const G = 9.81;
export const P = {
  mass: 0.970, t_max: 5.50, k_yaw: 0.012, x_arm: 0.055, y_arm: 0.1115,
  I: [0.0046, 0.0040, 0.0075],
  att_wn: 14.0, att_zeta: 0.9, yaw_kp: 16.0, yaw_kd: 8.0,
  pos_kp: 5.0, pos_kd: 4.0, alt_kp: 8.0, alt_kd: 5.0, alt_ki: 2.0,
  max_tilt: 0.40, max_acc: 4.0,
  g_per_w: 2.3, avionics_w: 6.0, pack_wh: 18.9, usable: 0.80, wheel_w: 1.0,
};
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
export class FlightController {
  constructor() { this.iz = 0; }
  reset() { this.iz = 0; }
  // pos, vel: world [3]; quat [w,x,y,z]; omega: body [3]
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
export class Battery {
  constructor() { this.whUsed = 0; this.th = P.mass * G / 4; this.ph = (this.th / G * 1000) / P.g_per_w; }
  power(f, driving = 0) { return f.reduce((s, t) => s + this.ph * Math.pow(Math.max(t, 0) / this.th, 1.5), 0) + P.avionics_w + driving * 4 * P.wheel_w; }
  step(dt, f, driving = 0) { this.lastW = this.power(f, driving); this.whUsed += this.lastW * dt / 3600; return this.pct(); }
  pct() { return Math.max(0, 100 * (1 - this.whUsed / (P.pack_wh * P.usable))); }
  hoverMinutes() { return P.pack_wh * P.usable / (4 * this.ph + P.avionics_w) * 60; }
}
