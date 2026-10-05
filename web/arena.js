// Coastal Agentics arena viewer: plain JS, no build step. Loads the engine (web/pkg, built
// from engine-wasm by scripts/build-wasm.sh) and renders its JSON state on a canvas.
// Engine coordinates are Y-up with angles counter-clockwise; the canvas is Y-down,
// so we flip Y and negate angles when drawing.
//
// Games: Tank (default) and Racing (?game=racing). Tank has Watch + Customize tabs.
// Tank URL: ?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4 (or legacy ?a=&b=).
// Racing URL: ?game=racing&seed=42 — four Nyborgs on the Ring (follower/cutter/blocker/follower).
// Viewer extras: tab=customize, speed=1|2|4, paused=1, t=<ticks to skip on load>,
// sprites=nyborg|classic (Nyborgs are the default tank sprites).
import init, { WasmMatch, WasmRace, tankCatalogJson, snapLoadout, canonicalTankQuery } from "./pkg/engine_wasm.js";
import {
  PLACEHOLDER_BOTS, TRI, specFromQuery, queryFromSpec, pointToWeights, weightsToPoint,
  loadoutWeights, readout, presetName,
} from "./tank-ui.js";
import { drawNyborg, hairColor, idleBob, facingFromHeading } from "./nyborg.js";
import {
  defaultRaceBuilds, raceSpecFromQuery, queryFromRaceSpec,
  drawTrack, drawRaceCar, drawFinishBanner, hairForCar, RACE_HAIR,
} from "./race-ui.js";

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
// Copy of `spec` for the match that is actually running. The Watch tab's cards and result
// line describe this, never `spec`, which Customize edits before the match restarts.
let running = null;
// Agent sprite mode: Nyborgs are the default (M5 first slice). Classic tanks via ?sprites=classic.
let spriteMode = "nyborg";
// Last facing (+1/-1) per agent id so flips stay stable near vertical headings.
const facingById = new Map();
// Racing: static track geometry + setup for HUD / finish banner.
let track = null;
let raceSetup = null;

const behaviorName = (key) => (catalog.behaviors.find(([k]) => k === key) || [key, key])[1];
const pretty = (l) => l.replaceAll("-", "/");
const isRacing = () => spec?.game === "racing";

function setSpeed(s) {
  speed = s;
  document.querySelectorAll(".speed").forEach((b) => b.classList.toggle("on", Number(b.dataset.speed) === s));
  if (isRacing()) syncUrl();
}

function setPlaying(p) {
  playing = p;
  $("play").textContent = p ? "Pause" : "Play";
  lastTime = null;
}

function setTab(t) {
  if (isRacing() && t === "customize") t = "watch";
  tab = t;
  for (const name of ["watch", "customize"]) {
    $(`tab-${name}`).setAttribute("aria-selected", String(name === t));
    $(`panel-${name}`).hidden = name !== t;
  }
  if (t === "customize") renderCustomize();
  // Builds edited in Customize take effect when you come back to Watch.
  if (t === "watch" && spec && !isRacing() && queryFromSpec(spec) !== runningQuery) restart();
  syncUrl();
}

function setGame(g) {
  const next = g === "racing" ? "racing" : "tank";
  document.querySelectorAll(".game").forEach((b) => b.classList.toggle("on", b.dataset.game === next));
  $("tank-bots").hidden = next === "racing";
  $("tank-sprites").hidden = next === "racing";
  $("tank-tabs").hidden = next === "racing";
  $("race-hud").hidden = next !== "racing";
  $("blurb").textContent = next === "racing"
    ? "The deterministic Rust engine, compiled to WebAssembly, running a Ring race with Nyborg drivers. Same link, same race, every time."
    : "The deterministic Rust engine, compiled to WebAssembly, running a Tank Arena duel live at 60 ticks per second. Same link, same match, every time.";
  $("arena").setAttribute("aria-label", next === "racing" ? "Ring race with Nyborg drivers" : "Tank Arena match with Nyborg agents");
  if (next === "racing") {
    if (!isRacing()) spec = { game: "racing", seed: (spec?.seed || "42"), builds: defaultRaceBuilds() };
    setTab("watch");
  } else if (isRacing()) {
    try { spec = defaultSpec(); }
    catch { spec = { mode: "tank", seed: "42", tanks: [{ behavior: "kiter", loadout: "5-3-1" }, { behavior: "charger", loadout: "4-1-4" }] }; }
  }
  restart();
}

function syncUrl() {
  if (!spec) return;
  if (isRacing()) {
    history.replaceState(null, "", "?" + queryFromRaceSpec(spec, {
      speed: speed !== 1 ? speed : 0,
      paused: playing ? 0 : 1,
    }));
  } else {
    history.replaceState(null, "", "?" + queryFromSpec(spec, { tab: tab === "customize" ? "customize" : "" }));
    $("cz-link").textContent = $("cz-link").href = location.href.replace(/[?&]tab=customize/, "");
  }
}

// --- match lifecycle -------------------------------------------------------------------

function restart() {
  facingById.clear();
  try {
    match?.free();
    match = null;
    track = null;
    raceSetup = null;
    if (isRacing()) {
      match = WasmRace.fromBuilds(spec.seed, JSON.stringify(spec.builds || defaultRaceBuilds()));
      track = JSON.parse(match.trackJson());
      raceSetup = JSON.parse(match.setupJson());
      setup = raceSetup;
    } else if (spec.mode === "tank") {
      match = WasmMatch.tank(queryFromSpec(spec));
      setup = JSON.parse(match.setupJson());
    } else {
      match = new WasmMatch(spec.seed, spec.bots[0], spec.bots[1]);
      setup = null;
    }
  } catch (e) {
    // No match runs for a spec the engine rejects (e.g. a bad seed): drop the old one
    // entirely, so its canvas, cards and result can't pass for the requested setup.
    match = null;
    state = null;
    setup = null;
    track = null;
    raceSetup = null;
    running = null;
    runningQuery = null;
    $("clock").textContent = "No match";
    syncControls();
    syncUrl();
    draw();
    $("result").textContent = String(e.message || e);
    return;
  }
  running = JSON.parse(JSON.stringify(spec));
  runningQuery = isRacing() ? queryFromRaceSpec(spec) : queryFromSpec(spec);
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
  if (isRacing()) {
    refreshRace();
    return;
  }
  const secs = (state.tick / TICK_HZ).toFixed(1);
  const limit = (state.max_ticks / TICK_HZ).toFixed(0);
  $("clock").textContent = `Tick ${state.tick} · ${secs}s / ${limit}s`;
  const o = state.outcome;
  if (o) {
    const why = { last_standing: "last tank standing", all_destroyed: "both destroyed", tick_limit: "time limit" }[o.reason] || o.reason;
    const who = (i) => (running.mode === "tank" ? `${behaviorName(running.tanks[i].behavior)} ${pretty(running.tanks[i].loadout)}` : running.bots[i]);
    // Name both running builds, so the line always says which match ended.
    const side = (i) => `${TEAM_NAMES[i]} (${who(i)})`;
    const at = `${(o.ticks / TICK_HZ).toFixed(1)}s`;
    $("result").textContent = o.winner === null
      ? `Draw (${why}) at ${at}: ${side(0)} vs ${side(1)}`
      : `${side(o.winner)} beats ${side(1 - o.winner)} — ${why}, ${at}`;
  }
}

function refreshRace() {
  const hz = track?.tick_hz || TICK_HZ;
  const secs = (state.tick / hz).toFixed(1);
  const limit = (state.max_ticks / hz).toFixed(0);
  const lapsTotal = state.cars?.[0]?.laps_total ?? track?.laps ?? 3;
  // Show the leader's completed laps (max among cars).
  const lapDone = Math.max(0, ...(state.cars || []).map((c) => c.lap || 0));
  $("clock").textContent = `Lap ${Math.min(lapDone + 1, lapsTotal)}/${lapsTotal} · ${secs}s / ${limit}s`;
  renderRaceHud();
  const o = state.outcome || (state.over ? JSON.parse(match.outcomeJson()) : null);
  if (o && o !== null) {
    const winner = o.winner == null ? null : (raceSetup?.cars?.[o.winner]?.name || `Car ${o.winner}`);
    const at = `${(o.ticks / hz).toFixed(1)}s`;
    $("result").textContent = winner == null
      ? `Race over (${o.reason}) at ${at}`
      : `${winner} wins — ${o.reason}, ${at}`;
  }
}

function renderRaceHud() {
  const el = $("race-hud");
  if (!el || !state?.cars) { if (el) el.innerHTML = ""; return; }
  const order = [...state.cars].sort((a, b) => (a.placing ?? 99) - (b.placing ?? 99));
  const places = order.map((c) => {
    const name = raceSetup?.cars?.[c.id]?.name || `Car ${c.id}`;
    const beh = raceSetup?.cars?.[c.id]?.behavior || "";
    const hair = hairForCar(c.id);
    const done = c.finished ? " · finished" : "";
    return `<span class="place"><span class="swatch" style="background:${hair}"></span><strong>${c.placing}.</strong> ${name} <span class="muted">(${beh}${done})</span></span>`;
  }).join("");
  el.innerHTML = places;
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
  if (!isRacing()) {
    $("cz-seed").value = spec.seed;
    for (const i of [0, 1]) $(`bot${i}`).value = spec.mode === "tank" ? spec.tanks[i].behavior : spec.bots[i];
  }
  renderWatchCards();
}

function onBotChange(i) {
  if (isRacing()) return;
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
  const shown = running || spec; // the running match; the request if none could start
  if (shown.game === "racing") {
    const cars = raceSetup?.cars || shown.builds.map((b, i) => ({
      behavior: b.behavior?.id || "follower",
      name: (b.behavior?.id || "follower")[0].toUpperCase() + (b.behavior?.id || "follower").slice(1),
      setup: "3-3-3",
      training: "Scripted",
    }));
    el.innerHTML = cars.map((c, i) => {
      const hair = hairForCar(i);
      return `<div class="tankcard"><h3><span style="color:${hair}">${c.name}</span>${trainingBadge()}</h3>
        <div>Build ${c.setup} (P/T/G) · yarn ${RACE_HAIR[i % RACE_HAIR.length]} · ${c.behavior}</div></div>`;
    }).join("");
    $("watch-note").innerHTML = "<strong>Follower</strong> tracks the racing line; <strong>Cutter</strong> dives inside; <strong>Blocker</strong> sits wide and disrupts. Finished Nyborgs coast as ghosts.";
    return;
  }
  if (shown.mode !== "tank") {
    el.innerHTML = [0, 1].map((i) => `<div class="tankcard"><h3><span class="team${i}">${TEAM_NAMES[i]}: ${shown.bots[i]}</span><span class="badge">Placeholder</span></h3>
      <div class="muted">Engine built-in bot on the engine's random-spawn duel, default stats.</div></div>`).join("");
    $("watch-note").innerHTML = "<strong>Chaser</strong> drives at the enemy and fires when aligned; <strong>Wanderer</strong> steers randomly (seeded). Chaser (blue) vs Wanderer (orange) with seed S reproduces <code>engine-cli --seed S</code>.";
    return;
  }
  el.innerHTML = [0, 1].map((i) => {
    const t = shown.tanks[i], opp = shown.tanks[1 - i];
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
  if (isRacing()) { drawRace(); return; }
  if (!state) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    return;
  }
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

function drawRace() {
  if (!track) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    return;
  }
  const W = track.width, H = track.height;
  if (canvas.width !== W || canvas.height !== H) { canvas.width = W; canvas.height = H; }
  drawTrack(ctx, track);
  if (!state?.cars) return;
  // Draw finished (ghost) cars first so live leaders paint on top.
  const order = [...state.cars].sort((a, b) => Number(a.finished) - Number(b.finished));
  for (const c of order) {
    const prev = facingById.get(c.id) ?? 1;
    const facing = drawRaceCar(ctx, track, c, { facingPrev: prev, tick: state.tick });
    facingById.set(c.id, facing);
  }
  if (state.over || state.outcome) {
    const outcome = state.outcome || JSON.parse(match.outcomeJson());
    drawFinishBanner(ctx, track, raceSetup, outcome);
  }
}

function setSpriteMode(mode) {
  spriteMode = mode === "classic" ? "classic" : "nyborg";
  document.querySelectorAll(".sprites").forEach((b) => b.classList.toggle("on", b.dataset.sprites === spriteMode));
}

function drawTank(t, Y) {
  if (spriteMode === "classic") drawClassicTank(t, Y);
  else drawNyborgTank(t, Y);
}

function drawNyborgTank(t, Y) {
  const color = TEAM_COLORS[t.team] || "#ccc";
  const r = TANK_RADIUS;
  const prev = facingById.get(t.id) ?? 1;
  const facing = facingFromHeading(t.heading, prev);
  facingById.set(t.id, facing);
  const bob = t.alive ? idleBob(state?.tick) : 0;

  ctx.save();
  ctx.translate(t.x, Y(t.y));
  ctx.globalAlpha = t.alive ? 1 : 0.3;

  // Nyborg body (replaces the classic hull). Hair colour is team-primary; flip on heading x.
  drawNyborg(ctx, { hair: hairColor(t.team), facing, bob, scale: 0.72 });

  // Compact aim indicator (turret barrel) so aim direction stays readable.
  ctx.save();
  ctx.rotate(-t.turret);
  ctx.fillStyle = "#e6e8ee";
  ctx.fillRect(4, -2, r + 6, 4);
  ctx.fillStyle = color;
  ctx.beginPath(); ctx.arc(0, 0, 4.5, 0, Math.PI * 2); ctx.fill();
  ctx.restore();

  drawHpOrX(t, r);
  ctx.restore();
}

function drawClassicTank(t, Y) {
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

  drawHpOrX(t, r);
  ctx.restore();
}

function drawHpOrX(t, r) {
  // HP bar (screen-aligned, above the agent).
  if (t.alive) {
    const w = 36, frac = Math.max(0, t.hp / t.max_hp);
    ctx.fillStyle = "#000a";
    ctx.fillRect(-w / 2, -r - 14, w, 5);
    ctx.fillStyle = frac > 0.5 ? "#3ddc84" : frac > 0.25 ? "#ffcc33" : "#ff4d4d";
    ctx.fillRect(-w / 2, -r - 14, w * frac, 5);
  } else {
    ctx.strokeStyle = "#fff";
    ctx.lineWidth = 3;
    ctx.beginPath(); ctx.moveTo(-8, -8); ctx.lineTo(8, 8); ctx.moveTo(8, -8); ctx.lineTo(-8, 8); ctx.stroke();
  }
}

// --- loop ----------------------------------------------------------------------------

function frame(now) {
  const racingOver = isRacing() && (state?.over || state?.outcome);
  if (match && playing && !state?.outcome && !racingOver) {
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
document.querySelectorAll(".sprites").forEach((b) => b.addEventListener("click", () => setSpriteMode(b.dataset.sprites)));
document.querySelectorAll(".game").forEach((b) => b.addEventListener("click", () => setGame(b.dataset.game)));

await init();
catalog = JSON.parse(tankCatalogJson());
fillBotSelect($("bot0"));
fillBotSelect($("bot1"));
const params = new URLSearchParams(location.search);
const wantRacing = params.get("game") === "racing";
try {
  if (wantRacing) spec = raceSpecFromQuery(location.search);
  else spec = specFromQuery(location.search, canonicalTankQuery);
} catch (e) {
  spec = wantRacing ? raceSpecFromQuery("?game=racing&seed=42") : defaultSpec();
  queueMicrotask(() => { $("result").textContent = `Bad link (${e.message || e}); showing the default match.`; });
}
// Sync game chrome without restarting twice.
document.querySelectorAll(".game").forEach((b) => b.classList.toggle("on", b.dataset.game === (wantRacing ? "racing" : "tank")));
$("tank-bots").hidden = wantRacing;
$("tank-sprites").hidden = wantRacing;
$("tank-tabs").hidden = wantRacing;
$("race-hud").hidden = !wantRacing;
if (wantRacing) {
  $("blurb").textContent = "The deterministic Rust engine, compiled to WebAssembly, running a Ring race with Nyborg drivers. Same link, same race, every time.";
  $("arena").setAttribute("aria-label", "Ring race with Nyborg drivers");
}
if (params.has("speed")) setSpeed(Number(params.get("speed")) || 1);
if (params.get("paused") === "1") setPlaying(false);
if (params.get("sprites") === "classic") setSpriteMode("classic");
else setSpriteMode("nyborg");
restart();
if (!wantRacing && params.get("tab") === "customize") setTab("customize");
// Optional: jump ahead N ticks on load (?t=600), handy for sharing a moment of a match.
const skip = Number(params.get("t") || 0);
if (skip > 0 && match) { match.step(skip); refresh(); }
requestAnimationFrame(frame);
// For headless checks: current state, setup, spec and the engine's state hash.
window.__arena = {
  get state() { return state; },
  get setup() { return setup; },
  get spec() { return spec; },
  get running() { return running; },
  get sprites() { return spriteMode; },
  get track() { return track; },
  get game() { return isRacing() ? "racing" : "tank"; },
  hash: () => (match && typeof match.stateHash === 'function' ? match.stateHash() : undefined),
};
