// Coastal Agentics arena viewer: plain JS, no build step. Loads the engine (web/pkg, built
// from engine-wasm by scripts/build-wasm.sh) and renders its JSON state on a canvas.
// Engine coordinates are Y-up with angles counter-clockwise; the canvas is Y-down,
// so we flip Y and negate angles when drawing.
//
// Two tabs: Watch (play a match) and Customize (each tank's behavior and 9-point build).
// The match lives in the URL: ?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4 (Tank Arena)
// or the legacy ?seed=42&a=Chaser&b=Wanderer (built-in bots, same as engine-cli).
// Viewer extras: tab=customize, speed=1|2|4, paused=1, t=<ticks to skip on load>.
import init, { WasmMatch, tankCatalogJson, snapLoadout, canonicalTankQuery } from "./pkg/engine_wasm.js";
import {
  PLACEHOLDER_BOTS, TRI, specFromQuery, queryFromSpec, pointToWeights, weightsToPoint,
  loadoutWeights, readout, presetName,
} from "./tank-ui.js";

const TICK_HZ = 60;
const TEAM_COLORS = ["#4da3ff", "#ff6b3d"];
const TEAM_NAMES = ["Blue", "Orange"];
const TANK_RADIUS = 16; // engine TankParams::default().radius (shared by every tank)
const TRAINING_TIP = "Hand-written policy. Once the Phase 3 evolution loop runs, this shows the generation, win rate vs the other policies and a fitness sparkline.";

const $ = (id) => document.getElementById(id);
const canvas = $("arena");
const ctx = canvas.getContext("2d");

let catalog = null;
let spec = null; // { mode: "tank", seed, tanks: [{behavior, loadout}, ...] } or { mode: "bots", seed, bots }
let lastTanks = null; // remembered tank setup while the built-in bots are showing
let match = null;
let setup = null; // WasmMatch.setupJson() for tank matches
let state = null;
let playing = true;
let speed = 1;
let carry = 0; // fractional ticks owed to the sim
let lastTime = null;
let tab = "watch";
let runningQuery = null; // queryFromSpec of the match on the Watch tab

const behaviorName = (key) => (catalog.behaviors.find(([k]) => k === key) || [key, key])[1];
const pretty = (l) => l.replaceAll("-", "/");

function setSpeed(s) {
  speed = s;
  document.querySelectorAll(".speed").forEach((b) => b.classList.toggle("on", Number(b.dataset.speed) === s));
}

function setPlaying(p) {
  playing = p;
  $("play").textContent = p ? "Pause" : "Play";
  lastTime = null;
}

function setTab(t) {
  tab = t;
  for (const name of ["watch", "customize"]) {
    $(`tab-${name}`).setAttribute("aria-selected", String(name === t));
    $(`panel-${name}`).hidden = name !== t;
  }
  if (t === "customize") renderCustomize();
  // Builds edited in Customize take effect when you come back to Watch.
  if (t === "watch" && spec && queryFromSpec(spec) !== runningQuery) restart();
  syncUrl();
}

function syncUrl() {
  if (!spec) return;
  history.replaceState(null, "", "?" + queryFromSpec(spec, { tab: tab === "customize" ? "customize" : "" }));
  $("cz-link").textContent = $("cz-link").href = location.href.replace(/[?&]tab=customize/, "");
}

// --- match lifecycle -------------------------------------------------------------------

function restart() {
  try {
    match?.free();
    match = null;
    if (spec.mode === "tank") {
      match = WasmMatch.tank(queryFromSpec(spec));
      setup = JSON.parse(match.setupJson());
    } else {
      match = new WasmMatch(spec.seed, spec.bots[0], spec.bots[1]);
      setup = null;
    }
  } catch (e) {
    match = null;
    $("result").textContent = String(e.message || e);
    return;
  }
  runningQuery = queryFromSpec(spec);
  carry = 0;
  lastTime = null;
  $("result").textContent = "";
  syncControls();
  syncUrl();
  refresh();
  if (!playing) draw();
}

function refresh() {
  state = JSON.parse(match.stateJson());
  const secs = (state.tick / TICK_HZ).toFixed(1);
  const limit = (state.max_ticks / TICK_HZ).toFixed(0);
  $("clock").textContent = `Tick ${state.tick} · ${secs}s / ${limit}s`;
  const o = state.outcome;
  if (o) {
    const why = { last_standing: "last tank standing", all_destroyed: "both destroyed", tick_limit: "time limit" }[o.reason] || o.reason;
    const who = (i) => (spec.mode === "tank" ? `${behaviorName(spec.tanks[i].behavior)} ${pretty(spec.tanks[i].loadout)}` : spec.bots[i]);
    $("result").textContent = o.winner === null
      ? `Draw (${why}) at ${(o.ticks / TICK_HZ).toFixed(1)}s`
      : `${TEAM_NAMES[o.winner]} (${who(o.winner)}) wins — ${why}, ${(o.ticks / TICK_HZ).toFixed(1)}s`;
  }
}

// --- Watch tab controls ------------------------------------------------------------------

function fillBotSelect(sel) {
  sel.innerHTML = "";
  for (const [key, name] of catalog.behaviors) sel.add(new Option(name, key));
  const g = document.createElement("optgroup");
  g.label = "Built-in bots (engine-cli)";
  for (const b of PLACEHOLDER_BOTS) g.appendChild(new Option(b, b));
  sel.appendChild(g);
}

function syncControls() {
  $("seed").value = spec.seed;
  $("cz-seed").value = spec.seed;
  for (const i of [0, 1]) $(`bot${i}`).value = spec.mode === "tank" ? spec.tanks[i].behavior : spec.bots[i];
  renderWatchCards();
}

function onBotChange(i) {
  const v = $(`bot${i}`).value;
  const isBot = PLACEHOLDER_BOTS.includes(v);
  if (isBot) {
    if (spec.mode === "tank") lastTanks = spec.tanks;
    const bots = spec.mode === "bots" ? [...spec.bots] : ["Chaser", "Wanderer"];
    bots[i] = v;
    spec = { mode: "bots", seed: spec.seed, bots };
  } else {
    const tanks = (spec.mode === "tank" ? spec.tanks : lastTanks || JSON.parse(JSON.stringify(defaultSpec().tanks))).map((t) => ({ ...t }));
    tanks[i].behavior = v;
    spec = { mode: "tank", seed: spec.seed, tanks };
  }
  restart();
}

function onSeedChange(value) {
  spec = { ...spec, seed: value.trim() || "0" };
  restart();
}

function defaultSpec() {
  return specFromQuery("?" + catalog.default_query, canonicalTankQuery);
}

function trainingBadge() {
  return `<span class="badge" title="${TRAINING_TIP}">Scripted</span>`;
}

function renderWatchCards() {
  const el = $("watch-cards");
  if (spec.mode !== "tank") {
    el.innerHTML = [0, 1].map((i) => `<div class="tankcard"><h3><span class="team${i}">${TEAM_NAMES[i]}: ${spec.bots[i]}</span><span class="badge">Placeholder</span></h3>
      <div class="muted">Engine built-in bot on the engine's random-spawn duel, default stats.</div></div>`).join("");
    $("watch-note").innerHTML = "<strong>Chaser</strong> drives at the enemy and fires when aligned; <strong>Wanderer</strong> steers randomly (seeded). Chaser (blue) vs Wanderer (orange) with seed S reproduces <code>engine-cli --seed S</code>.";
    return;
  }
  el.innerHTML = [0, 1].map((i) => {
    const t = spec.tanks[i], opp = spec.tanks[1 - i];
    const r = readout(catalog, t.loadout, opp.loadout);
    const preset = presetName(catalog, t.loadout);
    return `<div class="tankcard"><h3><span class="team${i}">${TEAM_NAMES[i]}: ${behaviorName(t.behavior)}</span>${trainingBadge()}</h3>
      <div>Build ${pretty(t.loadout)}${preset ? ` (${preset})` : ""} · ${r.damage} dmg · reload ${r.reloadSec} s · ${r.speed} u/s · ${r.hp} HP · kills in ${r.hitsToKill}</div></div>`;
  }).join("");
  $("watch-note").innerHTML = "<strong>Charger</strong> rushes and trades up close; <strong>Kiter</strong> circle-strafes at 250–350 u; <strong>Sniper</strong> holds a far spot with a sight line and fires only when precisely aimed. Set builds in the Customize tab.";
}

// --- Customize tab -------------------------------------------------------------------------

function ensureTankSpec() {
  if (spec.mode !== "tank") {
    spec = { mode: "tank", seed: spec.seed, tanks: (lastTanks || defaultSpec().tanks).map((t) => ({ ...t })) };
  }
}

function renderCustomize() {
  ensureTankSpec();
  const el = $("cz-cards");
  if (!el.dataset.built) {
    el.innerHTML = [0, 1].map((i) => `<div class="tankcard" id="cz-card${i}">
      <h3><span class="team${i}">${TEAM_NAMES[i]}</span>${trainingBadge()}</h3>
      <label>Behavior <select id="cz-beh${i}">${catalog.behaviors.map(([k, n]) => `<option value="${k}">${n}</option>`).join("")}</select></label>
      <svg class="tri" id="cz-tri${i}" viewBox="0 0 220 214" role="slider" aria-label="${TEAM_NAMES[i]} loadout triangle" tabindex="0">
        <polygon points="${[TRI.A, TRI.S, TRI.D].map((p) => p.join(",")).join(" ")}" fill="#141a26" stroke="#3a4560" />
        ${catalog.loadouts.map((l) => { const [x, y] = weightsToPoint(loadoutWeights(l)); return `<circle cx="${x}" cy="${y}" r="3" fill="#3a4560" data-l="${l}" />`; }).join("")}
        <circle id="cz-dot${i}" r="7" fill="${TEAM_COLORS[i]}" stroke="#fff" stroke-width="2" />
        <text x="110" y="9" text-anchor="middle">Attack</text>
        <text x="4" y="206">Speed</text>
        <text x="216" y="206" text-anchor="end">Defense</text>
      </svg>
      <div class="presets" id="cz-presets${i}">${catalog.presets.map(([n, l]) => `<button type="button" data-l="${l}">${n} ${pretty(l)}</button>`).join("")}</div>
      <dl class="readout" id="cz-read${i}"></dl>
      <p class="muted" style="margin:8px 0 0">Training: <strong>Scripted</strong> (hand-written). Generation, win rate and a fitness sparkline appear here once evolution runs.</p>
    </div>`).join("");
    el.dataset.built = "1";
    for (const i of [0, 1]) wireCard(i);
  }
  for (const i of [0, 1]) {
    const t = spec.tanks[i], opp = spec.tanks[1 - i];
    $(`cz-beh${i}`).value = t.behavior;
    const [x, y] = weightsToPoint(loadoutWeights(t.loadout));
    $(`cz-dot${i}`).setAttribute("cx", x);
    $(`cz-dot${i}`).setAttribute("cy", y);
    $(`cz-tri${i}`).setAttribute("aria-valuetext", pretty(t.loadout));
    $(`cz-tri${i}`).dataset.loadout = t.loadout;
    document.querySelectorAll(`#cz-presets${i} button`).forEach((b) => b.classList.toggle("on", b.dataset.l === t.loadout));
    const r = readout(catalog, t.loadout, opp.loadout);
    const preset = presetName(catalog, t.loadout);
    $(`cz-read${i}`).innerHTML = `
      <dt>Build</dt><dd>${pretty(t.loadout)} (A/S/D)${preset ? ` · ${preset}` : ""}</dd>
      <dt>Damage</dt><dd>${r.damage} per hit</dd>
      <dt>Reload</dt><dd>${r.reloadSec} s between shots</dd>
      <dt>Max speed</dt><dd>${r.speed} u/s · turns ${r.turnDegPerSec}°/s</dd>
      <dt>HP</dt><dd>${r.hp}</dd>
      <dt>Hits to kill</dt><dd>${r.hitsToKill} (${r.damage} dmg vs ${TEAM_NAMES[1 - i]}'s ${r.oppHp} HP)</dd>`;
  }
  $("cz-seed").value = spec.seed;
  $("cz-error").textContent = "";
  syncUrl();
}

function setLoadout(i, loadout) {
  if (spec.tanks[i].loadout === loadout) return;
  spec.tanks[i] = { ...spec.tanks[i], loadout };
  renderCustomize();
}

function wireCard(i) {
  $(`cz-beh${i}`).addEventListener("change", (e) => {
    spec.tanks[i] = { ...spec.tanks[i], behavior: e.target.value };
    renderCustomize();
  });
  $(`cz-presets${i}`).addEventListener("click", (e) => {
    const l = e.target.closest("button")?.dataset.l;
    if (l) setLoadout(i, l);
  });
  const svg = $(`cz-tri${i}`);
  const pick = (ev) => {
    const pt = svg.createSVGPoint();
    pt.x = ev.clientX; pt.y = ev.clientY;
    const p = pt.matrixTransform(svg.getScreenCTM().inverse());
    const [a, s, d] = pointToWeights(p.x, p.y);
    setLoadout(i, snapLoadout(a, s, d)); // the Rust snap: always one of the 19
  };
  svg.addEventListener("pointerdown", (ev) => { svg.setPointerCapture(ev.pointerId); pick(ev); });
  svg.addEventListener("pointermove", (ev) => { if (svg.hasPointerCapture(ev.pointerId)) pick(ev); });
  // Keyboard: arrows move one point between stats (stays within the 19 by snapping).
  svg.addEventListener("keydown", (ev) => {
    const w = loadoutWeights(spec.tanks[i].loadout);
    const step = 1 / 6;
    const moves = { ArrowUp: [step, -step / 2, -step / 2], ArrowLeft: [-step / 2, step, -step / 2], ArrowRight: [-step / 2, -step / 2, step], ArrowDown: [-step, step / 2, step / 2] };
    const m = moves[ev.key];
    if (!m) return;
    ev.preventDefault();
    setLoadout(i, snapLoadout(w[0] + m[0], w[1] + m[1], w[2] + m[2]));
  });
}

function watchThis() {
  ensureTankSpec();
  const seed = $("cz-seed").value.trim() || "0";
  try {
    canonicalTankQuery(queryFromSpec({ ...spec, seed }));
  } catch (e) {
    $("cz-error").textContent = String(e.message || e);
    return;
  }
  spec = { ...spec, seed };
  setPlaying(true);
  restart();
  setTab("watch");
}

// --- drawing -------------------------------------------------------------------------

function draw() {
  if (!state) return;
  const { width: W, height: H } = state;
  if (canvas.width !== W || canvas.height !== H) { canvas.width = W; canvas.height = H; }
  const Y = (y) => H - y;

  ctx.fillStyle = "#0e1219";
  ctx.fillRect(0, 0, W, H);
  ctx.strokeStyle = "#161c27";
  ctx.lineWidth = 1;
  for (let x = 50; x < W; x += 50) { ctx.beginPath(); ctx.moveTo(x + 0.5, 0); ctx.lineTo(x + 0.5, H); ctx.stroke(); }
  for (let y = 50; y < H; y += 50) { ctx.beginPath(); ctx.moveTo(0, y + 0.5); ctx.lineTo(W, y + 0.5); ctx.stroke(); }

  ctx.fillStyle = "#2a3244";
  ctx.strokeStyle = "#3a4560";
  for (const r of state.obstacles) {
    ctx.fillRect(r.x, Y(r.y + r.h), r.w, r.h);
    ctx.strokeRect(r.x + 0.5, Y(r.y + r.h) + 0.5, r.w - 1, r.h - 1);
  }

  for (const p of state.projectiles) {
    const color = TEAM_COLORS[p.team] || "#fff";
    ctx.strokeStyle = color;
    ctx.globalAlpha = 0.35;
    ctx.lineWidth = 2;
    ctx.beginPath(); ctx.moveTo(p.x - p.vx * 2, Y(p.y - p.vy * 2)); ctx.lineTo(p.x, Y(p.y)); ctx.stroke();
    ctx.globalAlpha = 1;
    ctx.fillStyle = "#fff";
    ctx.beginPath(); ctx.arc(p.x, Y(p.y), 3, 0, Math.PI * 2); ctx.fill();
  }

  for (const t of state.tanks) drawTank(t, Y);
}

function drawTank(t, Y) {
  const color = TEAM_COLORS[t.team] || "#ccc";
  const r = TANK_RADIUS;
  ctx.save();
  ctx.translate(t.x, Y(t.y));
  ctx.globalAlpha = t.alive ? 1 : 0.3;

  // Hull (rotated with heading; canvas angles are clockwise, so negate).
  ctx.save();
  ctx.rotate(-t.heading);
  ctx.fillStyle = color;
  ctx.strokeStyle = "#0b0d12";
  ctx.lineWidth = 2;
  ctx.beginPath(); ctx.rect(-r, -r * 0.8, r * 2, r * 1.6); ctx.fill(); ctx.stroke();
  ctx.fillStyle = "rgba(0,0,0,.35)"; // treads
  ctx.fillRect(-r, -r * 0.8, r * 2, r * 0.35);
  ctx.fillRect(-r, r * 0.45, r * 2, r * 0.35);
  ctx.fillStyle = "rgba(255,255,255,.6)"; // nose marker
  ctx.beginPath(); ctx.moveTo(r, 0); ctx.lineTo(r - 6, -4); ctx.lineTo(r - 6, 4); ctx.fill();
  ctx.restore();

  // Turret.
  ctx.save();
  ctx.rotate(-t.turret);
  ctx.fillStyle = "#e6e8ee";
  ctx.fillRect(0, -2.5, r + 8, 5);
  ctx.beginPath(); ctx.arc(0, 0, r * 0.45, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = color;
  ctx.beginPath(); ctx.arc(0, 0, r * 0.28, 0, Math.PI * 2); ctx.fill();
  ctx.restore();

  // HP bar (screen-aligned, above the tank).
  if (t.alive) {
    const w = 36, frac = Math.max(0, t.hp / t.max_hp);
    ctx.fillStyle = "#000a";
    ctx.fillRect(-w / 2, -r - 12, w, 5);
    ctx.fillStyle = frac > 0.5 ? "#3ddc84" : frac > 0.25 ? "#ffcc33" : "#ff4d4d";
    ctx.fillRect(-w / 2, -r - 12, w * frac, 5);
  } else {
    ctx.strokeStyle = "#fff";
    ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(-8, -8); ctx.lineTo(8, 8); ctx.moveTo(8, -8); ctx.lineTo(-8, 8); ctx.stroke();
  }
  ctx.restore();
}

// --- loop ----------------------------------------------------------------------------

function frame(now) {
  if (match && playing && !state?.outcome) {
    if (lastTime !== null) {
      // Cap the catch-up so a backgrounded tab doesn't fast-forward the whole match.
      carry += Math.min((now - lastTime) / 1000, 0.1) * TICK_HZ * speed;
      const n = Math.floor(carry);
      if (n > 0) { carry -= n; match.step(n); refresh(); }
    }
    lastTime = now;
  }
  draw();
  requestAnimationFrame(frame);
}

$("play").addEventListener("click", () => setPlaying(!playing));
$("restart").addEventListener("click", restart);
$("seed").addEventListener("change", (e) => onSeedChange(e.target.value));
$("bot0").addEventListener("change", () => onBotChange(0));
$("bot1").addEventListener("change", () => onBotChange(1));
$("tab-watch").addEventListener("click", () => setTab("watch"));
$("tab-customize").addEventListener("click", () => setTab("customize"));
$("watch-this").addEventListener("click", watchThis);
$("cz-seed").addEventListener("input", (e) => { spec = { ...spec, seed: e.target.value.trim() || "0" }; syncUrl(); });
document.querySelectorAll(".speed").forEach((b) => b.addEventListener("click", () => setSpeed(Number(b.dataset.speed))));

await init();
catalog = JSON.parse(tankCatalogJson());
fillBotSelect($("bot0"));
fillBotSelect($("bot1"));
const params = new URLSearchParams(location.search);
try {
  spec = specFromQuery(location.search, canonicalTankQuery);
} catch (e) {
  spec = defaultSpec();
  queueMicrotask(() => { $("result").textContent = `Bad link (${e.message || e}); showing the default match.`; });
}
if (params.has("speed")) setSpeed(Number(params.get("speed")) || 1);
if (params.get("paused") === "1") setPlaying(false);
restart();
if (params.get("tab") === "customize") setTab("customize");
// Optional: jump ahead N ticks on load (?t=600), handy for sharing a moment of a match.
const skip = Number(params.get("t") || 0);
if (skip > 0 && match) { match.step(skip); refresh(); }
requestAnimationFrame(frame);
// For headless checks: current state, setup, spec and the engine's state hash.
window.__arena = {
  get state() { return state; },
  get setup() { return setup; },
  get spec() { return spec; },
  hash: () => match?.stateHash(),
};
