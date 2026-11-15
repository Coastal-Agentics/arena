// Read the Python parity JSON on stdin, add JS reference outputs, write stdout.
// The JS controller has no acceleration feedforward; those cases omit `js`.
import { Battery, FlightController, mix, quatToRpy, wrap } from "../../../../assets/tbd/reference/tbd_control.js";

const raw = await new Response(process.stdin).text();
const doc = JSON.parse(raw);
for (const c of doc.controller) {
  const aff = c.a_ff;
  const hasFf = aff.some((v) => v !== 0);
  if (hasFf) continue;
  const fc = new FlightController();
  for (const s of c.steps) {
    const r = fc.step(c.dt, s.pos, s.quat, s.vel, s.omega, s.p_des, s.v_des, s.yaw_des);
    s.js = { u: r.u, f: r.f, rpy: r.rpy };
  }
}
for (const m of doc.mix) {
  m.js = mix(m.F, m.tx, m.ty, m.tz);
}
for (const w of doc.wrap) w.js = wrap(w.a);
for (const q of doc.quat) q.js = quatToRpy(...q.q);
{
  const bat = new Battery();
  for (const s of doc.battery.steps) {
    s.js_power = bat.power(s.thrusts, s.driving);
    s.js_pct = bat.step(doc.battery.dt, s.thrusts, s.driving);
  }
  doc.battery.js_hover_minutes = bat.hoverMinutes();
}
process.stdout.write(JSON.stringify(doc, null, 2) + "\n");
