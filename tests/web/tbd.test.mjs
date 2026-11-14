import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { G, P, W_MAX, STOW, quatToRpy, mix, wrap, FlightController, Battery, readInput, deadzone,
  KEY_AXES, KEY_ACTIONS, PAD_ACTIONS, MODES, createJsController, stairScore, stepHeight, createHopMetrics } from "../../web/tbd/controller.js";
import { pickController, pilotFromInput, buildObs, createWasmController, modeName, PILOT_TAKEOFF, OBS_LEN_MANUAL } from "../../web/tbd/controller-wasm.js";

const LEVEL = [1, 0, 0, 0];
const state = (o = {}) => ({ dt: 0.002, pos: [0, 0, 0], quat: LEVEL, vel: [0, 0, 0], omega: [0, 0, 0], agl: 0, fold: STOW, battery: 100, ...o });
const still = { fwd: 0, turn: 0, strafe: 0, climb: 0, yaw: 0 };
const near = (a, b, eps = 1e-9) => assert.ok(Math.abs(a - b) <= eps, `${a} vs ${b}`);

test("math helpers: level quaternion, yaw-only quaternion, wrap", () => {
  assert.deepEqual(quatToRpy(...LEVEL), [0, 0, 0]);
  const yaw = 0.7, q = [Math.cos(yaw / 2), 0, 0, Math.sin(yaw / 2)];
  near(quatToRpy(...q)[2], yaw, 1e-12);
  near(Math.abs(wrap(3 * Math.PI)), Math.PI, 1e-12); near(wrap(2 * Math.PI + 0.25), 0.25, 1e-12); near(wrap(-0.5), -0.5, 1e-12);
});

test("mixer: pure collective splits evenly and clamps to max thrust", () => {
  const hover = P.mass * G;
  for (const f of mix(hover, 0, 0, 0)) near(f, hover / 4, 1e-12);
  for (const f of mix(1000, 0, 0, 0)) assert.equal(f, P.t_max);
  for (const f of mix(-5, 0, 0, 0)) assert.equal(f, 0);
});

test("flight controller: at the setpoint it commands hover thrust; altitude hold converges", () => {
  const fc = new FlightController();
  const r = fc.step(0.002, [0, 0, 1], LEVEL, [0, 0, 0], [0, 0, 0], [0, 0, 1]);
  for (const f of r.f) near(f, P.mass * G / 4, 1e-9);
  for (const u of r.u) assert.ok(u > 0 && u < 1);
  // vertical point-mass plant: z'' = sum(f)/m - g
  fc.reset(); let z = 0.3, vz = 0;
  for (let i = 0; i < 6000; i++) { const { f } = fc.step(0.002, [0, 0, z], LEVEL, [0, 0, vz], [0, 0, 0], [0, 0, 1]); vz += (f.reduce((a, b) => a + b) / P.mass - G) * 0.002; z += vz * 0.002; }
  near(z, 1, 0.01); near(vz, 0, 0.01);
});

test("battery drains faster in hover than idle, and estimates hover minutes", () => {
  const a = new Battery(), b = new Battery();
  for (let i = 0; i < 1000; i++) { a.step(0.01, [0, 0, 0, 0]); b.step(0.01, Array(4).fill(P.mass * G / 4)); }
  assert.ok(a.pct() > b.pct() && b.pct() < 100);
  const m = new Battery().hoverMinutes(); assert.ok(m > 1 && m < 30, `hover minutes ${m}`);
});

test("key mapping: WASD drive and turn, Space/Shift climb, Q/E yaw, opposite keys cancel", () => {
  assert.deepEqual(readInput(new Set()), still);
  assert.equal(readInput(new Set(["KeyW"])).fwd, 1);
  assert.equal(readInput(new Set(["KeyS"])).fwd, -1);
  assert.equal(readInput(new Set(["KeyW", "KeyS"])).fwd, 0);
  assert.deepEqual(readInput(new Set(["KeyD"])), { ...still, turn: 1, strafe: 1 });
  assert.deepEqual(readInput(new Set(["KeyA"])), { ...still, turn: -1, strafe: -1 });
  assert.equal(readInput(new Set(["Space"])).climb, 1);
  assert.equal(readInput(new Set(["ShiftLeft"])).climb, -1);
  assert.equal(readInput(new Set(["ShiftLeft", "ShiftRight"])).climb, -1, "clamped");
  assert.equal(readInput(new Set(["KeyQ"])).yaw, 1);
  assert.equal(readInput(new Set(["KeyE"])).yaw, -1);
  assert.equal(readInput(new Set(["KeyW", "ArrowUp"])).fwd, 1, "clamped");
  assert.deepEqual(KEY_ACTIONS, { KeyF: "fly", KeyG: "hop", KeyR: "reset", KeyC: "camera", KeyP: "pause", KeyH: "help" });
  assert.equal(PAD_ACTIONS[0], "fly"); assert.equal(PAD_ACTIONS[2], "hop");
  for (const code of Object.keys(KEY_ACTIONS)) assert.ok(!KEY_AXES[code], `${code} is an action, not an axis`);
});

test("gamepad and touch mapping", () => {
  assert.equal(deadzone(0.05), 0); assert.equal(deadzone(0.5), 0.5);
  const pad = readInput(new Set(), { lx: 0.5, ly: -1, rx: 0.4, ry: -0.5, lt: 0, rt: 0 });
  assert.deepEqual(pad, { fwd: 1, turn: 0.5, strafe: 0.5, climb: 0.5, yaw: -0.4 });
  const t = readInput(new Set(), null, { x: 0.6, y: -0.8, climb: -1 });
  assert.deepEqual(t, { fwd: 0.8, turn: 0.6, strafe: 0.6, climb: -1, yaw: 0 });
  assert.deepEqual(readInput(new Set(), null, { yaw: 1 }), { ...still, yaw: 1, turn: -1 }, "Turn L button");
});

test("controller step: interface shape and ground skid steer", () => {
  const c = createJsController(); c.reset(state());
  assert.equal(c.name, "js"); assert.equal(c.mode, "ground");
  const cmd = c.step(state(), still);
  assert.deepEqual(Object.keys(cmd).sort(), ["auto", "ctrl", "drive", "driving", "fold", "force", "mode", "thrust", "wind"]);
  assert.deepEqual(cmd.ctrl, [0, 0, 0, 0, 0, 0, 0, 0, STOW], "ctrl is in <actuator> order");
  assert.deepEqual(cmd.wind, [0, 0, 0]);
  assert.deepEqual(cmd.drive, [0, 0]); assert.deepEqual(cmd.thrust, [0, 0, 0, 0]); assert.equal(cmd.fold, STOW);
  assert.deepEqual(c.step(state(), { ...still, fwd: 1 }).drive, [W_MAX, W_MAX]);
  assert.deepEqual(c.step(state(), { ...still, turn: 1 }).drive, [W_MAX, -W_MAX]);
  assert.deepEqual(c.step(state(), { ...still, fwd: 1, turn: 1 }).drive, [W_MAX, 0], "clamped to W_MAX");
  assert.equal(c.step(state(), { ...still, fwd: -0.5 }).driving, 0.5);
  assert.deepEqual(c.step(state(), { ...still, climb: 1, yaw: 1 }).drive, [0, 0], "climb/yaw do nothing on the ground");
});

test("controller step: fly unfolds, lifts off 30 cm, lands and folds", () => {
  const c = createJsController(); c.reset(state());
  c.press("fly"); let cmd = c.step(state(), still);
  assert.equal(cmd.mode, "deploying"); assert.equal(cmd.fold, 0); assert.deepEqual(cmd.drive, [0, 0]);
  cmd = c.step(state({ fold: 0.01 }), still);
  assert.equal(c.mode, "flight");
  cmd = c.step(state({ fold: 0 }), still);
  assert.ok(cmd.thrust.reduce((a, b) => a + b) > 0, "thrust on in flight");
  const avg = cmd.force.reduce((a, b) => a + b) / 4; assert.ok(avg > P.mass * G / 4, "climbing to the +30 cm target");
  assert.ok(MODES.includes(cmd.mode));
  c.press("fly"); assert.equal(c.mode, "landing");
  for (let i = 0; i < 200; i++) cmd = c.step(state({ fold: 0, agl: 0.005 }), still);
  assert.equal(c.mode, "stowing"); assert.equal(cmd.fold, STOW);
  c.step(state({ fold: STOW }), still); assert.equal(c.mode, "ground");
});

test("controller step: empty battery forces a landing; hop autopilot runs its phases", () => {
  const c = createJsController(); c.reset(state(), "flight");
  c.step(state({ fold: 0, agl: 0.5, pos: [0, 0, 0.5], battery: 0 }), still);
  assert.equal(c.mode, "landing");
  c.startAuto(state());
  assert.equal(c.auto, "drive");
  assert.deepEqual(c.step(state(), still).drive, [24, 24]);
  c.press("fly"); assert.equal(c.mode, "ground", "fly is ignored during the autopilot");
  c.step(state({ pos: [0.45, 0, 0] }), still); assert.equal(c.auto, "brake");
  let cmd; for (let i = 0; i < 300; i++) cmd = c.step(state({ pos: [0.45, 0, 0] }), still);
  assert.equal(c.auto, "deploy"); assert.equal(cmd.fold, 0);
  for (let i = 0; i < 300; i++) cmd = c.step(state({ pos: [0.45, 0, 0], fold: 0 }), still);
  assert.equal(c.auto, "climb"); assert.equal(c.mode, "flight");
});

test("step is deterministic for the same state/input sequence (parity reference)", () => {
  const run = () => { const c = createJsController(); c.reset(state()); c.press("fly"); const out = [];
    for (let i = 0; i < 50; i++) out.push(c.step(state({ fold: i > 5 ? 0 : 1, pos: [0, 0, i * 0.001] }), { ...still, fwd: 0.3, yaw: 0.2 }));
    return JSON.stringify(out); };
  assert.equal(run(), run());
});

test("stair-hop score matches the games/tbd formula", () => {
  assert.equal(stepHeight(0.5), 0); assert.equal(stepHeight(0.7), 0.17); assert.equal(stepHeight(1.9), 0.68);
  assert.equal(stairScore({ landed: false, progressX: 1.15 }), 50);
  near(stairScore({ landed: true, duration: 10, maxTiltDeg: 20, batteryUsed: 5, minClearance: 0.3 }), 1000 - 250 - 80 - 40 + 60, 1e-9);
  near(stairScore({ landed: true, duration: 10, maxTiltDeg: 0, batteryUsed: 0, minClearance: 2 }), 1000 - 250 + 100, 1e-9);
  const m = createHopMetrics(); m.update("climb", [1.0, 0, 0.6], [1, 0, 0, 0], 3); m.update("touchdown", [2.3, 0, 0.68], [1, 0, 0, 0], 8); m.update("stow", [2.3, 0, 0.68], [1, 0, 0, 0], 8.4);
  const r = m.result(10, 95, 2.8); assert.ok(r.landed); near(r.minClearance, 0.6 - 0.34, 1e-12); assert.equal(r.landedAt, 8.4);
});

test("wasm adapter: pilot channels, observation layout, mode names", () => {
  assert.deepEqual(pilotFromInput("ground", { ...still, fwd: 1, turn: 1 }), [1, -1, 0, 0], "D turns right = slow the right side");
  assert.deepEqual(pilotFromInput("ground", { ...still, turn: -1, climb: 1 }), [0, 1, 0, 0]);
  assert.deepEqual(pilotFromInput("flight", { ...still, fwd: -1, yaw: 1 }), [-1, 1, 0, 0], "Q yaws left");
  assert.deepEqual(pilotFromInput("flight", { ...still, strafe: 1, climb: -1 }), [0, 0, -1, -1], "D strafes right = pilot_y -1; Shift descends");
  assert.deepEqual(pilotFromInput("flight", { ...still, strafe: 1, climb: 1 }, 20), [0, -1, 0, 0], "20-slot build: A/D yaw, no climb");
  const sensors = Array.from({ length: 12 }, (_, i) => i + 1);
  for (const n of [20, OBS_LEN_MANUAL]) {
    const obs = buildObs(new Float64Array(n), state({ sensors, pos: [13, 14, 15], vel: [16, 17, 18] }), [19, 20, 21, 22]);
    assert.deepEqual(Array.from(obs), Array.from({ length: n }, (_, i) => i + 1));
  }
  assert.equal(modeName("unfolding"), "deploying"); assert.equal(modeName("folding"), "stowing");
  for (const m of MODES) assert.equal(modeName(m), m);
});

test("wasm adapter: falls back to JS when web/pkg has no WasmTbd", async () => {
  assert.equal((await pickController("wasm", createJsController, { url: "data:text/javascript,export default 1" })).name, "js");
  assert.equal((await pickController(null, createJsController)).name, "js");
});

// ---- parity with the engine's WasmTbd (PR #132). Skips when web/pkg does not export it.
const pkgUrl = new URL("../../web/pkg/engine_wasm.js", import.meta.url);
async function loadPkg() {
  try {
    const mod = await import(pkgUrl.href);
    if (typeof mod.WasmTbd !== "function") return null;
    await mod.default({ module_or_path: readFileSync(new URL("../../web/pkg/engine_wasm_bg.wasm", import.meta.url)) });
    return mod;
  } catch { return null; }
}
const pkg = await loadPkg();
const skip = pkg ? false : "web/pkg does not export WasmTbd (needs #132)";
const manual = pkg && pkg.tbdObsLen() >= OBS_LEN_MANUAL;
const skip133 = skip || (manual ? false : "web/pkg has no strafe/climb channels (needs #133)");
const sensorsOf = (s) => [...s.omega, 0, 0, G, ...s.quat, s.agl, s.fold];
// Loose on purpose: these must hold on both the #132 and #133 builds. The control-law check runs with the
// #133 physics switches off (physics: false; a no-op on #132), the defaults-on check only bounds the gap.
const TOL = { drive: 1e-6, thrust: 0.08, thrustPhysics: 0.25 };

test("parity: manual ground sticks match the JS skid steer", { skip }, () => {
  for (const input of [{ ...still, fwd: 1 }, { ...still, turn: 1 }, { ...still, fwd: 0.5, turn: -0.5 }, { ...still, fwd: -1 }]) {
    const js = createJsController(); js.reset(state());
    const w = createWasmController(pkg, { seed: 1, wind: false });
    const s = state(); s.sensors = sensorsOf(s);
    const a = js.step(s, input).ctrl, b = w.step(s, input).ctrl; w.free();
    for (let i = 4; i < 8; i++) near(b[i], a[i], TOL.drive);
    assert.equal(b[8], a[8]);
  }
});

test("parity: stair-hop script phases and hover thrust match the JS autopilot (wind off)", { skip }, () => {
  const js = createJsController(); js.startAuto(state());
  const w = createWasmController(pkg, { seed: 1, wind: false, physics: false }); w.startAuto(state());
  // a scripted pose sequence: drive to x 0.45, brake, unfold, then hover at the start of the climb
  const seq = [];
  for (let i = 0; i < 10; i++) seq.push(state({ pos: [0.04 * i, 0, 0] }));
  for (let i = 0; i < 300; i++) seq.push(state({ pos: [0.45, 0, 0] }));
  for (let i = 0; i < 260; i++) seq.push(state({ pos: [0.45, 0, 0], fold: 0 }));
  for (let i = 0; i < 50; i++) seq.push(state({ pos: [0.45, 0, 0.05], fold: 0 }));
  let compared = 0, prev = js.auto;
  for (const s of seq) {
    s.sensors = sensorsOf(s);
    const a = js.step(s, still), b = w.step(s, still);
    assert.equal(w.auto, js.auto, `phase at x ${s.pos[0]}`);
    for (let i = 4; i < 9; i++) near(b.ctrl[i], a.ctrl[i], TOL.drive);
    // Known one-tick difference: on the deploy -> climb tick the engine already flies, the JS reference waits a tick.
    if (js.auto === prev) { for (let i = 0; i < 4; i++) near(b.ctrl[i], a.ctrl[i], TOL.thrust); compared++; }
    prev = js.auto;
  }
  assert.equal(js.auto, "climb"); assert.ok(compared > 600, `compared ${compared}`);
  w.free();
});

test("parity: with the engine's estimated physics on, hover thrust stays near the JS command", { skip }, () => {
  const js = createJsController(); js.reset(state(), "flight");
  const w = createWasmController(pkg, { seed: 1, wind: false });
  const s = state({ fold: 0 }); s.sensors = sensorsOf(s);
  w.press("fly"); w.step(s, still);
  for (let i = 0; i < 400; i++) w.step(s, still);
  assert.equal(w.mode, "flight");
  const h = state({ fold: 0, pos: [0, 0, 0.6], agl: 0.6 }); h.sensors = sensorsOf(h);
  const a = js.step(h, still).ctrl, b = w.step(h, still).ctrl;
  const sum = (v) => v.slice(0, 4).reduce((x, y) => x + y);
  assert.ok(Math.abs(sum(b) - sum(a)) / 4 < TOL.thrustPhysics, `mean thrust ${sum(a) / 4} vs ${sum(b) / 4}`);
  w.free();
});

test("parity: wasm adapter takes off and lands through the sentinels", { skip }, () => {
  const w = createWasmController(pkg, { seed: 1, wind: true });
  const s = state(); s.sensors = sensorsOf(s);
  w.step(s, still); assert.equal(w.mode, "ground");
  w.press("fly"); w.step(s, still); assert.equal(w.mode, "deploying");
  const up = state({ fold: 0 }); up.sensors = sensorsOf(up);
  for (let i = 0; i < 300; i++) w.step(up, still);
  assert.equal(w.mode, "flight");
  const cmd = w.step(up, still); assert.equal(cmd.wind.length, 3); assert.ok(cmd.wind.some((v) => v !== 0), "seeded wind is drawn");
  w.press("fly"); w.step(up, still); assert.equal(w.mode, "landing");
  for (let i = 0; i < 300; i++) w.step(up, still);
  assert.equal(w.mode, "stowing", "page sends the fold sentinel once down");
  assert.equal(PILOT_TAKEOFF, 2); w.free();
});

test("parity (#133): manual climb, descend, strafe channels and landing abort", { skip: skip133 }, () => {
  const fly = () => {
    const w = createWasmController(pkg, { seed: 1, wind: false, physics: false });
    const g = state(); g.sensors = sensorsOf(g); w.step(g, still); w.press("fly"); w.step(g, still);
    const up = state({ fold: 0, pos: [0, 0, 0.6], agl: 0.6 }); up.sensors = sensorsOf(up);
    for (let i = 0; i < 400; i++) w.step(up, still);
    assert.equal(w.mode, "flight"); return [w, up];
  };
  const thrust = (input) => { const [w, up] = fly(); let c; for (let i = 0; i < 50; i++) c = w.step(up, input); w.free(); return c.ctrl.slice(0, 4); };
  const sum = (v) => v.reduce((x, y) => x + y);
  const hover = thrust(still), climb = thrust({ ...still, climb: 1 }), down = thrust({ ...still, climb: -1 });
  assert.ok(sum(climb) > sum(hover) + 0.05 && sum(down) < sum(hover) - 0.05, `climb ${sum(climb)} hover ${sum(hover)} down ${sum(down)}`);
  // strafe right (D): roll right, so the left ducts (FL, RL) push harder than the right ones (FR, RR)
  const right = thrust({ ...still, strafe: 1 });
  assert.ok(right[0] + right[2] > right[1] + right[3] + 0.02, `strafe right ${right}`);
  const left = thrust({ ...still, strafe: -1 });
  assert.ok(left[1] + left[3] > left[0] + left[2] + 0.02, `strafe left ${left}`);
  // landing -> back to flight with F while still airborne
  const [w, up] = fly(); w.press("fly"); w.step(up, still); assert.equal(w.mode, "landing");
  w.press("fly"); w.step(up, still); assert.equal(w.mode, "flight"); w.free();
});

test("page copy: physics sim with estimated parameters, working name TBD, relative paths in every file", () => {
  const dir = new URL("../../web/tbd/", import.meta.url);
  const html = readFileSync(new URL("index.html", dir), "utf8");
  assert.match(html, /physics sim/i); assert.match(html, /estimated parameters/i); assert.match(html, /working name/i);
  assert.match(html, /@mujoco\/mujoco@4\.0\.1\//, "MuJoCo pinned"); assert.match(html, /three@0\.187\.0\//, "three.js pinned");
  assert.doesNotMatch(html, /(src|href)="\//, "no root-absolute asset paths");
  for (const f of readdirSync(dir)) {
    if (!/\.(html|js|css)$/.test(f)) continue;
    const s = readFileSync(new URL(f, dir), "utf8");
    assert.doesNotMatch(s, /(src|href)="\//, f);
  }
});
