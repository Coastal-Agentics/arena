// TBD sim: MuJoCo 4.0.1 (WebAssembly) physics + three.js rendering. Plain JS, no build step.
// Physics sim of a hybrid drive-and-fly vehicle with ESTIMATED parameters. TBD is a working name.
// The controller lives in controller.js (JS reference) behind a state-in / commands-out step,
// so a Rust/wasm controller can drop in via controller-wasm.js (?ctrl=wasm).
import { MUJOCO_WASM_URL, MUJOCO_WASM_BYTES, HUD_KEY, ROTATE_KEY, isMobileLike, pickDpr, renderQuality, layoutFor, viewShift,
  fetchWithProgress, formatMB, loadFlag, saveFlag, createFpsMeter } from './phone.js?v=20261114d';
import { createJsController, createHopMetrics, Battery, readInput, deadzone, quatToRpy, KEY_ACTIONS, PAD_ACTIONS, KEY_AXES } from './controller.js?v=20261114d';
import { pickController } from './controller-wasm.js?v=20261114d';

const $ = id => document.getElementById(id);
const status = msg => { $('loading-msg').textContent = msg; console.log('[tbd]', msg); };
const setProgress = (fraction, text) => {
  const pct = Math.round(fraction * 100);
  $('progress-bar').style.width = `${pct}%`; $('progress-bar').parentElement.setAttribute('aria-valuenow', String(pct));
  if (text !== undefined) $('loading-bytes').textContent = text;
};
const coarse = matchMedia('(pointer: coarse)').matches || new URLSearchParams(location.search).get('touch') === '1';
const MOBILE = isMobileLike({ coarse, width: innerWidth, height: innerHeight });
document.body.classList.toggle('mobile', MOBILE);

// ---------------------------------------------------------------- no page zoom / scroll (iOS too)
for (const ev of ['gesturestart', 'gesturechange', 'gestureend']) document.addEventListener(ev, e => e.preventDefault(), { passive: false });
document.addEventListener('dblclick', e => e.preventDefault(), { passive: false });
document.addEventListener('touchmove', e => { if (e.touches.length > 1 || !e.target.closest('.help')) e.preventDefault(); }, { passive: false });
let lastTouchEnd = 0;   // double-tap zoom on older iOS
document.addEventListener('touchend', e => { const now = performance.now(); if (now - lastTouchEnd < 350 && !e.target.closest('a, button')) e.preventDefault(); lastTouchEnd = now; }, { passive: false });
const params = new URLSearchParams(location.search);
const MESHES = ['body_shell', 'body_slate', 'body_face', 'body_eyes', 'body_dark', 'wheel_tire', 'wheel_hub',
  'rotorA_duct', 'rotorA_motor', 'rotorA_prop', 'rotorB_duct', 'rotorB_motor', 'rotorB_prop'];
window.TBD = { ready: false };
var layoutReady = false;   // set once layout() can run (touch UI may appear before that)

// ---------------------------------------------------------------- load MuJoCo + model (relative paths)
// Stream the 11 MB MuJoCo wasm ourselves for a progress bar, then hand the bytes to the module (wasmBinary).
status('Loading MuJoCo physics…');
const wasmUrl = (() => { try { return new URL('mujoco.wasm', import.meta.resolve('mujoco')).href; } catch { return MUJOCO_WASM_URL; } })();
const libs = Promise.all([import('three'), import('three/addons/controls/OrbitControls.js'), import('three/addons/utils/BufferGeometryUtils.js'), import('mujoco')]);
let wasmBinary = null;
try {
  wasmBinary = await fetchWithProgress(wasmUrl, ({ loaded, total, exact, fraction }) =>
    setProgress(fraction, `${formatMB(loaded)} of ${exact ? '' : 'about '}${formatMB(total)}`), { approx: MUJOCO_WASM_BYTES });
} catch (err) { console.warn('[tbd] progress download failed; letting MuJoCo fetch it', err); }
status('Starting physics…');
const [THREE, { OrbitControls }, { toCreasedNormals }, { default: loadMujoco }] = await libs;
const mj = await loadMujoco(wasmBinary ? { wasmBinary } : {});
wasmBinary = null;
status('Loading TBD model…');
mj.FS.mkdir('/tbd'); mj.FS.mkdir('/tbd/meshes');
mj.FS.writeFile('/tbd/tbd.xml', await (await fetch('model/tbd.xml?v=20261114d')).text());
await Promise.all(MESHES.map(async n => {
  const buf = await (await fetch(`model/meshes/${n}.stl?v=20261114d`)).arrayBuffer();
  mj.FS.writeFile(`/tbd/meshes/${n}.stl`, new Uint8Array(buf));
}));
const model = mj.MjModel.from_xml_path('/tbd/tbd.xml');
const data = new mj.MjData(model);
const OBJ = mj.mjtObj;
const id = (type, name) => mj.mj_name2id(model, type.value, name);
const ACT_ORDER = ['thrust_FL', 'thrust_FR', 'thrust_RL', 'thrust_RR', 'drive_FL', 'drive_FR', 'drive_RL', 'drive_RR', 'fold'];
const ACT = {}; for (const n of ACT_ORDER) ACT[n] = id(OBJ.mjOBJ_ACTUATOR, n);
const ACT_IDS = ACT_ORDER.map(n => ACT[n]);
const BODY = id(OBJ.mjOBJ_BODY, 'tbd');
const THR = ['thrust_FL', 'thrust_FR', 'thrust_RL', 'thrust_RR'].map(n => ACT[n]);
const FOLD_Q = model.jnt_qposadr[id(OBJ.mjOBJ_JOINT, 'fold_FL')];
const RANGE_ADR = model.sensor_adr[id(OBJ.mjOBJ_SENSOR, 'range_down')];
const THR_SITES = ['FL', 'FR', 'RL', 'RR'].map(c => id(OBJ.mjOBJ_SITE, `thrust_${c}`));
const EYES = id(OBJ.mjOBJ_GEOM, 'eyes');
const DT = model.opt.timestep;
const ctlOpts = () => ({ seed: params.get('seed') || 1, wind: params.get('wind') !== '0', dt: DT });
let ctl = await pickController(params.get('ctrl'), createJsController, ctlOpts());

// ---------------------------------------------------------------- three.js scene from MuJoCo geoms
const QUALITY = renderQuality(MOBILE && params.get('quality') !== 'high');
const SEG = QUALITY.lite ? 16 : 32;
const renderer = new THREE.WebGLRenderer({ canvas: $('view'), antialias: QUALITY.antialias, powerPreference: 'high-performance' });
renderer.setPixelRatio(pickDpr({ devicePixelRatio, mobile: MOBILE }));
renderer.shadowMap.enabled = QUALITY.shadows; renderer.shadowMap.type = THREE.PCFShadowMap;
renderer.outputColorSpace = THREE.SRGBColorSpace;
const scene = new THREE.Scene();
scene.background = new THREE.Color(0xe9f0f7);
scene.fog = new THREE.Fog(0xe9f0f7, 6, 16);
const camera = new THREE.PerspectiveCamera(50, 1, 0.01, 50);
camera.up.set(0, 0, 1); camera.position.set(-1.0, -1.1, 0.6);
const orbit = new OrbitControls(camera, renderer.domElement);
orbit.target.set(0, 0, 0.08); orbit.enableDamping = true; orbit.enabled = false;
scene.add(new THREE.HemisphereLight(0xffffff, 0xb8c4d0, 1.3));
const sun = new THREE.DirectionalLight(0xffffff, 1.9);
sun.castShadow = QUALITY.shadows; if (QUALITY.shadows) sun.shadow.mapSize.set(QUALITY.shadowMap, QUALITY.shadowMap);
Object.assign(sun.shadow.camera, { left: -2.5, right: 2.5, top: 2.5, bottom: -2.5, near: 0.1, far: 12 });
sun.shadow.bias = -0.0004; scene.add(sun, sun.target);

function checkerTexture() {
  const c = document.createElement('canvas'); c.width = c.height = 256; const g = c.getContext('2d');
  for (let i = 0; i < 2; i++) for (let j = 0; j < 2; j++) { g.fillStyle = (i + j) % 2 ? '#d2d4d6' : '#e3e5e7'; g.fillRect(i * 128, j * 128, 128, 128); }
  g.strokeStyle = '#bfc4ca'; g.lineWidth = 2; g.strokeRect(0, 0, 256, 256);
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; t.repeat.set(24, 24);
  t.colorSpace = THREE.SRGBColorSpace; t.anisotropy = QUALITY.lite ? 1 : 8; return t;
}
const meshCache = {};
function meshGeometry(mid) {
  if (meshCache[mid]) return meshCache[mid];
  const va = model.mesh_vertadr[mid], vn = model.mesh_vertnum[mid], fa = model.mesh_faceadr[mid], fn = model.mesh_facenum[mid];
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(new Float32Array(model.mesh_vert.slice(3 * va, 3 * (va + vn))), 3));
  g.setIndex(Array.from(model.mesh_face.slice(3 * fa, 3 * (fa + fn))));
  return (meshCache[mid] = toCreasedNormals(g, 0.6));
}
const geomObjs = [];
for (let g = 0; g < model.ngeom; g++) {
  if (model.geom_group[g] > 2) continue;
  const type = model.geom_type[g], s = model.geom_size.slice(3 * g, 3 * g + 3);
  const mat = model.geom_matid[g];
  const rgba = mat >= 0 ? model.mat_rgba.slice(4 * mat, 4 * mat + 4) : model.geom_rgba.slice(4 * g, 4 * g + 4);
  const emission = mat >= 0 ? model.mat_emission[mat] : 0;
  const color = new THREE.Color().setRGB(rgba[0], rgba[1], rgba[2], THREE.SRGBColorSpace);
  const matArgs = { color, transparent: rgba[3] < 1, opacity: rgba[3], emissive: emission > 0 ? color : 0x000000, emissiveIntensity: emission };
  const m = QUALITY.lite ? new THREE.MeshLambertMaterial(matArgs) : new THREE.MeshStandardMaterial({ ...matArgs, roughness: 0.62, metalness: 0.05 });
  let geo;
  if (type === 0) { geo = new THREE.PlaneGeometry(2 * (s[0] || 20), 2 * (s[1] || 20)); m.map = checkerTexture(); m.color.set(0xffffff); m.roughness = 0.9; }
  else if (type === 2) geo = new THREE.SphereGeometry(s[0], SEG, SEG >> 1);
  else if (type === 3) { geo = new THREE.CapsuleGeometry(s[0], 2 * s[1], 8, 16); geo.rotateX(Math.PI / 2); }
  else if (type === 4) { geo = new THREE.SphereGeometry(1, SEG, SEG >> 1); geo.scale(s[0], s[1], s[2]); }
  else if (type === 5) { geo = new THREE.CylinderGeometry(s[0], s[0], 2 * s[1], SEG); geo.rotateX(Math.PI / 2); }
  else if (type === 6) geo = new THREE.BoxGeometry(2 * s[0], 2 * s[1], 2 * s[2]);
  else if (type === 7) geo = meshGeometry(model.geom_dataid[g]);
  else continue;
  const obj = new THREE.Mesh(geo, m);
  obj.castShadow = QUALITY.shadows && type !== 0; obj.receiveShadow = QUALITY.shadows; obj.matrixAutoUpdate = false;
  scene.add(obj); geomObjs.push([g, obj]);
}
const eyeObj = geomObjs.find(([g]) => g === EYES)?.[1];
// thrust "prop blur" discs at the duct thrust sites (visual only)
const discs = THR_SITES.map(() => {
  const d = new THREE.Mesh(new THREE.CircleGeometry(0.037, 32),
    new THREE.MeshBasicMaterial({ color: 0xffb547, transparent: true, opacity: 0, side: THREE.DoubleSide, depthWrite: false }));
  d.matrixAutoUpdate = false; scene.add(d); return d;
});
function setFrom(obj, pos, mat, i) {
  obj.matrix.set(mat[9 * i], mat[9 * i + 1], mat[9 * i + 2], pos[3 * i],
                 mat[9 * i + 3], mat[9 * i + 4], mat[9 * i + 5], pos[3 * i + 1],
                 mat[9 * i + 6], mat[9 * i + 7], mat[9 * i + 8], pos[3 * i + 2], 0, 0, 0, 1);
}
function syncScene() {
  const gp = data.geom_xpos, gm = data.geom_xmat;
  for (const [g, obj] of geomObjs) setFrom(obj, gp, gm, g);
  const sp = data.site_xpos, sm = data.site_xmat;
  THR_SITES.forEach((s, i) => { setFrom(discs[i], sp, sm, s); discs[i].material.opacity = 0.65 * Math.min(1, data.ctrl[THR[i]]); });
}

// ---------------------------------------------------------------- input: keyboard, gamepad, touch
const keys = new Set();
addEventListener('keydown', e => {
  if (['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.code)) e.preventDefault();
  if (!keys.has(e.code) && KEY_ACTIONS[e.code]) act(KEY_ACTIONS[e.code]);
  if (KEY_AXES[e.code] || KEY_ACTIONS[e.code]) keys.add(e.code);
});
addEventListener('keyup', e => keys.delete(e.code));
addEventListener('blur', () => { keys.clear(); touch.x = touch.y = touch.climb = touch.yaw = 0; for (const h in holds) holds[h] = false; });
let padPrev = {};
function readPad() {
  const pad = [...(navigator.getGamepads?.() || [])].find(p => p);
  if (!pad) return null;
  const now = {};
  for (const [btn, action] of Object.entries(PAD_ACTIONS)) {
    now[btn] = !!pad.buttons[btn]?.pressed;
    if (now[btn] && !padPrev[btn]) act(action);
  }
  padPrev = now;
  return { lx: deadzone(pad.axes[0]), ly: deadzone(pad.axes[1]), rx: deadzone(pad.axes[2] ?? 0), ry: deadzone(pad.axes[3] ?? 0), lt: pad.buttons[6]?.value || 0, rt: pad.buttons[7]?.value || 0 };
}

const touch = { x: 0, y: 0, climb: 0, yaw: 0 };
const holds = { climb: false, descend: false, yawl: false, yawr: false };
function setupTouch() {
  const ui = $('touch');
  const show = () => { ui.hidden = false; document.body.classList.add('has-touch'); if (layoutReady) layout(); };
  if (coarse) show();
  addEventListener('touchstart', show, { once: true, passive: true });
  const stick = $('stick'), knob = $('knob');
  let pid = null;
  const moveKnob = e => {
    const r = stick.getBoundingClientRect(), R = r.width / 2;
    let dx = e.clientX - (r.left + R), dy = e.clientY - (r.top + R);
    const n = Math.hypot(dx, dy); if (n > R) { dx *= R / n; dy *= R / n; }
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
    touch.x = deadzone(dx / R, 0.15); touch.y = deadzone(dy / R, 0.15);
  };
  stick.addEventListener('pointerdown', e => { pid = e.pointerId; stick.setPointerCapture(pid); moveKnob(e); e.preventDefault(); });
  stick.addEventListener('pointermove', e => { if (e.pointerId === pid) moveKnob(e); });
  const end = e => { if (e.pointerId !== pid) return; pid = null; knob.style.transform = ''; touch.x = touch.y = 0; };
  stick.addEventListener('pointerup', end); stick.addEventListener('pointercancel', end);
  for (const b of document.querySelectorAll('#touch button, .tools button')) {
    const hold = b.dataset.hold, action = b.dataset.act;
    b.addEventListener('pointerdown', e => {
      e.preventDefault(); b.classList.add('down');
      if (hold) { holds[hold] = true; b.setPointerCapture(e.pointerId); }
      if (action) act(action);
    });
    const up = () => { b.classList.remove('down'); if (hold) holds[hold] = false; };
    b.addEventListener('pointerup', up); b.addEventListener('pointercancel', up); b.addEventListener('lostpointercapture', up);
    b.addEventListener('contextmenu', e => e.preventDefault());
  }
}
setupTouch();

// ---------------------------------------------------------------- physics <-> controller
let bat = new Battery();
let paused = false, camMode = 'chase', simTime = 0, lastCmd = null;
const pos = () => [data.qpos[0], data.qpos[1], data.qpos[2]];
const quat = () => [data.qpos[3], data.qpos[4], data.qpos[5], data.qpos[6]];
const vel = () => [data.qvel[0], data.qvel[1], data.qvel[2]];
const omg = () => [data.qvel[3], data.qvel[4], data.qvel[5]];
const agl = () => { const r = data.sensordata[RANGE_ADR]; return r < 0 ? 9.99 : Math.max(0, r - 0.014); };
const sense = () => ({ dt: DT, pos: pos(), quat: quat(), vel: vel(), omega: omg(), agl: agl(), fold: data.qpos[FOLD_Q], battery: battPct(),
  sensors: sensors12() });
const sensors12 = () => { const sd = data.sensordata; return Array.from({ length: 12 }, (_, i) => sd[i]); };
const battPct = () => (lastCmd && lastCmd.battery !== undefined ? lastCmd.battery : bat.pct());

function reset(key = 'ground') {
  mj.mj_resetDataKeyframe(model, data, id(OBJ.mjOBJ_KEY, key)); mj.mj_forward(model, data);
  bat = new Battery(); simTime = 0; lastCmd = null;
  ctl.reset(sense(), key === 'ground' ? 'ground' : 'flight');
}
let hop = null;   // { metrics (JS), landedAt (wasm), result } for the current or last autopilot hop
function startAuto() { reset(); ctl.startAuto(sense()); hop = { metrics: createHopMetrics(), landedAt: null, result: null }; }
function act(action) {
  if (action === 'fly') ctl.press('fly');
  else if (action === 'hop') startAuto();
  else if (action === 'reset') reset();
  else if (action === 'camera') { camMode = camMode === 'chase' ? 'orbit' : 'chase'; orbit.enabled = camMode === 'orbit'; }
  else if (action === 'pause') paused = !paused;
  else if (action === 'help') $('help').classList.toggle('hidden');
}

function physicsStep(input) {
  const wasAuto = ctl.auto;
  const cmd = ctl.step(sense(), input);
  ACT_IDS.forEach((a, i) => (data.ctrl[a] = cmd.ctrl[i]));
  // world frame on body "tbd": force = wind (+ engine aero), torque = engine aero
  const xf = data.xfrc_applied, w = cmd.wind || [0, 0, 0], tq = cmd.torque || [0, 0, 0];
  for (let i = 0; i < 3; i++) { xf[6 * BODY + i] = w[i]; xf[6 * BODY + 3 + i] = tq[i]; }
  mj.mj_step(model, data); simTime += DT;
  bat.step(DT, cmd.force, cmd.driving); lastCmd = cmd;
  if (hop && !hop.result && wasAuto) trackHop(cmd);
}

function trackHop(cmd) {
  if (ctl.name === 'wasm') {
    const e = cmd.engine;
    if (e.landed_on_landing && hop.landedAt === null) hop.landedAt = simTime;
    if (ctl.hopResult && !ctl.auto) hop.result = { ...ctl.hopResult, landedAt: hop.landedAt, controller: 'wasm' };
  } else {
    hop.metrics.update(ctl.auto, pos(), quat(), simTime);
    if (!ctl.auto) hop.result = { ...hop.metrics.result(simTime, bat.pct(), pos()[0]), completed: true, controller: 'js' };
  }
}
const liveScore = () => {
  if (!hop) return null;
  if (hop.result) return hop.result.score;
  if (ctl.name === 'wasm' && lastCmd && lastCmd.engine) return lastCmd.engine.score;
  return hop.metrics.result(simTime, bat.pct(), pos()[0]).score;
};

// ---------------------------------------------------------------- camera + HUD
let camYaw = 0;
const camWant = new THREE.Vector3(), camLook = new THREE.Vector3(), camD = new THREE.Vector3();
function updateCamera(dt) {
  const p = pos();
  sun.position.set(p[0] - 1.2, p[1] - 2.0, p[2] + 6); sun.target.position.set(p[0], p[1], p[2]); sun.target.updateMatrixWorld();
  const mode = ctl.mode;
  if (camMode === 'chase') {
    const yaw = quatToRpy(...quat())[2];
    camYaw += Math.atan2(Math.sin(yaw - camYaw), Math.cos(yaw - camYaw)) * Math.min(1, dt * 2.5);
    const narrow = innerWidth < innerHeight ? 1.35 : 1;   // pull back on portrait phones
    const back = (mode === 'ground' ? 0.85 : 1.15) * narrow, up = (mode === 'ground' ? 0.38 : 0.55) * narrow;
    camWant.set(p[0] - back * Math.cos(camYaw) + 0.35 * Math.sin(camYaw), p[1] - back * Math.sin(camYaw) - 0.35 * Math.cos(camYaw), p[2] + up);
    camera.position.lerp(camWant, Math.min(1, dt * 4));
    orbit.target.lerp(camLook.set(p[0], p[1], p[2] + 0.07), Math.min(1, dt * 6));
    camera.lookAt(orbit.target);
  } else {
    camD.set(p[0], p[1], p[2] + 0.07).sub(orbit.target);
    orbit.target.add(camD); camera.position.add(camD); orbit.update();
  }
}
const MODE_LABEL = { ground: 'Ground · drive', deploying: 'Unfolding…', flight: 'Flight · assisted hover', landing: 'Landing…', stowing: 'Folding…' };
let rtf = 1;
function hud() {
  const p = pos(), v = vel(), batt = battPct(), mode = ctl.mode;
  $('hud-mode').textContent = (ctl.auto ? 'Autopilot hop · ' : '') + MODE_LABEL[mode];
  $('hud-mode').dataset.mode = mode;
  $('hud-alt').textContent = `${agl().toFixed(2)} m`;
  $('hud-z').textContent = `${p[2].toFixed(2)} m`;
  $('hud-speed').textContent = `${Math.hypot(v[0], v[1]).toFixed(2)} m/s`;
  $('hud-batt').textContent = `${batt.toFixed(1)} %`;
  $('hud-mini').textContent = `${agl().toFixed(1)} m · ${batt.toFixed(0)} %`;
  $('batt-bar').style.width = `${batt}%`;
  $('batt-bar').classList.toggle('low', batt < 25);
  const watts = lastCmd && lastCmd.powerW !== undefined ? lastCmd.powerW : bat.lastW;
  const rem = watts > 1 ? batt / 100 * bat.usableWh() / watts * 60 : 0;
  const volts = lastCmd && lastCmd.engine && lastCmd.engine.pack_voltage_v;
  $('hud-power').textContent = `${watts.toFixed(0)} W${volts ? ` · ${volts.toFixed(1)} V` : ''} · ${rem > 99 ? '>99' : rem.toFixed(1)} min left at this draw`;
  const sc = liveScore();
  $('hud-score-row').hidden = sc === null; $('hud-score').textContent = sc === null ? '' : `${sc.toFixed(1)}${hop && hop.result ? ' (final)' : ''}`;
  $('hud-fold').textContent = `${(data.qpos[FOLD_Q] * 180 / Math.PI).toFixed(0)}°`;
  (lastCmd ? lastCmd.thrust : [0, 0, 0, 0]).forEach((u, i) => ($(`thr${i}`).style.height = `${Math.round(u * 100)}%`));
  $('hud-time').textContent = `t ${simTime.toFixed(1)} s · ${rtf.toFixed(2)}× real time`;
  $('hud-ctl').textContent = ctl.name === 'wasm' ? `wasm controller (engine${ctl.hasManualControls ? '' : ', no climb/strafe'})` : 'JS controller';
  $('hud-ctl').dataset.ctl = ctl.name;
  $('fly-label').textContent = mode === 'ground' || mode === 'stowing' ? 'Fly' : 'Land';
}

// ---------------------------------------------------------------- main loop
// Layout: size the canvas, pick DPR, and shift the view so the vehicle sits in the band between the
// top UI (bar + HUD) and the thumb controls rather than under them.
let LAYOUT = layoutFor({ width: innerWidth, height: innerHeight, mobile: MOBILE });
function layout() {
  const w = innerWidth, h = innerHeight;
  LAYOUT = layoutFor({ width: w, height: h, mobile: MOBILE });
  document.body.dataset.orientation = LAYOUT.orientation;
  renderer.setPixelRatio(pickDpr({ devicePixelRatio, mobile: MOBILE }));
  renderer.setSize(w, h, false); camera.aspect = w / h;
  let top = 0, bottom = 0;
  if (MOBILE) {
    const hud = $('hud').getBoundingClientRect(), touchUi = $('touch');
    top = LAYOUT.orientation === 'portrait' ? hud.bottom : document.querySelector('.bar').getBoundingClientRect().bottom;
    if (!touchUi.hidden) bottom = h - Math.min(...[...touchUi.querySelectorAll('.stick, .pads')].map(e => e.getBoundingClientRect().top));
  }
  const dy = viewShift(h, top, bottom);
  if (dy) camera.setViewOffset(w, h, 0, dy, w, h); else camera.clearViewOffset();
  camera.updateProjectionMatrix();
  updateRotateHint();
}
function updateRotateHint() {
  $('rotate').hidden = !(LAYOUT.rotateHint && !$('touch').hidden && !loadFlag(localStorage, ROTATE_KEY));
}
$('rotate-x').addEventListener('click', () => { saveFlag(localStorage, ROTATE_KEY, true); updateRotateHint(); });
// HUD collapse, remembered across visits
function setHudCollapsed(c, save = true) {
  $('hud').classList.toggle('collapsed', c);
  $('hud-toggle').textContent = c ? '▸' : '▾';
  $('hud-toggle').setAttribute('aria-expanded', String(!c));
  $('hud-toggle').setAttribute('aria-label', c ? 'Expand status panel' : 'Collapse status panel');
  if (save) saveFlag(localStorage, HUD_KEY, c);
  layout();
}
$('hud-toggle').addEventListener('click', () => setHudCollapsed(!$('hud').classList.contains('collapsed')));
addEventListener('resize', layout); addEventListener('orientationchange', () => setTimeout(layout, 100));
layoutReady = true;
setHudCollapsed(loadFlag(localStorage, HUD_KEY), false);
reset();
let last = performance.now(), acc = 0, lastHud = 0, eyeAmber = null;
const fpsMeter = createFpsMeter(2000);
function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000); last = now;
  touch.climb = (holds.climb ? 1 : 0) - (holds.descend ? 1 : 0);
  touch.yaw = (holds.yawl ? 1 : 0) - (holds.yawr ? 1 : 0);
  const input = readInput(keys, readPad(), touch);
  if (!paused) {
    acc += dt; let n = 0;
    while (acc >= DT && n < 40) { physicsStep(input); acc -= DT; n++; }
    if (n >= 40) acc = 0;
    rtf = 0.9 * rtf + 0.1 * (n * DT / Math.max(dt, 1e-3));
  }
  const amber = ctl.mode !== 'ground';
  if (eyeObj && amber !== eyeAmber) { eyeAmber = amber; eyeObj.material.emissive.set(amber ? 0xffa53a : 0xf2f6ff); eyeObj.material.color.set(amber ? 0xffa53a : 0xf2f6ff); }
  syncScene(); updateCamera(dt);
  if (now - lastHud > 100) { lastHud = now; hud(); }   // DOM updates at 10 Hz keep the loop lean
  fpsMeter.tick(now);
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}
$('loading').classList.add('hidden');
// Test hooks (headless checks): window.TBD.press('KeyF'), .keys.add('KeyW'), .mode, .pos()
// Run one autopilot hop synchronously (no rendering) and return its metrics.
function runHop(maxTime = 20) {
  startAuto();
  const still = { fwd: 0, turn: 0, strafe: 0, climb: 0, yaw: 0 };
  while (!hop.result && simTime < maxTime) physicsStep(still);
  return { ...hop.result, endTime: simTime, pos: pos() };
}
async function useController(prefer, opts = {}) {
  const next = await pickController(prefer, createJsController, { ...ctlOpts(), ...opts });
  if (ctl.free) ctl.free();
  ctl = next; hop = null; reset(); return ctl.name;
}
window.TBD = { ready: true, get mode() { return ctl.mode; }, get auto() { return ctl.auto; }, get controller() { return ctl.name; }, pos, simTime: () => simTime,
  battery: () => battPct(), press: code => KEY_ACTIONS[code] && act(KEY_ACTIONS[code]), act, keys, touch, startAuto, reset, runHop, useController,
  get hop() { return hop && hop.result; },
  perf: () => ({ fps: fpsMeter.fps, rtf, dpr: renderer.getPixelRatio(), shadows: QUALITY.shadows, antialias: QUALITY.antialias, mobile: MOBILE, orientation: LAYOUT.orientation }),
  setHudCollapsed,
  vehicleScreen: () => { const p = pos(), v = new THREE.Vector3(p[0], p[1], p[2] + 0.05).project(camera); return [(v.x + 1) / 2 * innerWidth, (1 - v.y) / 2 * innerHeight]; }, get engine() { return lastCmd && lastCmd.engine; },
  setCam: m => { camMode = m; orbit.enabled = m === 'orbit'; } };
if (params.get('demo') === '1') startAuto();
requestAnimationFrame(frame);
