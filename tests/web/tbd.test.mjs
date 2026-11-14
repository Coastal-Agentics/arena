import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { G, P, W_MAX, STOW, quatToRpy, mix, wrap, FlightController, Battery, readInput, deadzone,
  KEY_AXES, KEY_ACTIONS, PAD_ACTIONS, MODES, createJsController } from "../../web/tbd/controller.js";
import { WASM_CONTROLLER_AVAILABLE, loadWasmController, pickController } from "../../web/tbd/controller-wasm.js";

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
  assert.deepEqual(t, { fwd: 0.8, turn: 0.6, strafe: 0, climb: -1, yaw: -0.6 });
});

test("controller step: interface shape and ground skid steer", () => {
  const c = createJsController(); c.reset(state());
  assert.equal(c.name, "js"); assert.equal(c.mode, "ground");
  const cmd = c.step(state(), still);
  assert.deepEqual(Object.keys(cmd).sort(), ["auto", "drive", "driving", "fold", "force", "mode", "thrust"]);
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

test("wasm adapter: stub until the Rust step lands, falls back to JS", async () => {
  assert.equal(WASM_CONTROLLER_AVAILABLE, false);
  assert.equal(await loadWasmController(), null);
  assert.equal((await pickController("wasm", createJsController)).name, "js");
  assert.equal((await pickController(null, createJsController)).name, "js");
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
