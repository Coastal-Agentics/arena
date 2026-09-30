// Coastal Agentics arena viewer: plain JS, no build step. Loads the engine (web/pkg, built
// from engine-wasm by scripts/build-wasm.sh) and renders its JSON state on a canvas.
// Engine coordinates are Y-up with angles counter-clockwise; the canvas is Y-down,
// so we flip Y and negate angles when drawing.
import init, { WasmMatch } from "./pkg/engine_wasm.js";

const TICK_HZ = 60;
const TEAM_COLORS = ["#4da3ff", "#ff6b3d"];
const TEAM_NAMES = ["Blue", "Orange"];
const TANK_RADIUS = 16; // engine TankParams::default().radius

const $ = (id) => document.getElementById(id);
const canvas = $("arena");
const ctx = canvas.getContext("2d");

let match = null;
let state = null;
let playing = true;
let speed = 1;
let carry = 0; // fractional ticks owed to the sim
let lastTime = null;

// URL params let links (and headless checks) pick a match: ?seed=7&a=Wanderer&b=Chaser&speed=4&t=600
const params = new URLSearchParams(location.search);
if (params.has("seed")) $("seed").value = params.get("seed");
if (params.has("a")) $("bot0").value = params.get("a");
if (params.has("b")) $("bot1").value = params.get("b");
if (params.has("speed")) setSpeed(Number(params.get("speed")) || 1);
if (params.get("paused") === "1") setPlaying(false);

function setSpeed(s) {
  speed = s;
  document.querySelectorAll(".speed").forEach((b) => b.classList.toggle("on", Number(b.dataset.speed) === s));
}

function setPlaying(p) {
  playing = p;
  $("play").textContent = p ? "Pause" : "Play";
  lastTime = null;
}

function restart() {
  const seed = $("seed").value.trim() || "0";
  try {
    match?.free();
    match = new WasmMatch(seed, $("bot0").value, $("bot1").value);
  } catch (e) {
    match = null;
    $("result").textContent = String(e.message || e);
    return;
  }
  carry = 0;
  lastTime = null;
  $("result").textContent = "";
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
    $("result").textContent = o.winner === null
      ? `Draw (${why}) at ${(o.ticks / TICK_HZ).toFixed(1)}s`
      : `${TEAM_NAMES[o.winner]} (${[$("bot0").value, $("bot1").value][o.winner]}) wins — ${why}, ${(o.ticks / TICK_HZ).toFixed(1)}s`;
  }
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
$("seed").addEventListener("change", restart);
$("bot0").addEventListener("change", restart);
$("bot1").addEventListener("change", restart);
document.querySelectorAll(".speed").forEach((b) => b.addEventListener("click", () => setSpeed(Number(b.dataset.speed))));

await init();
restart();
// Optional: jump ahead N ticks on load (?t=600), handy for sharing a moment of a match.
const skip = Number(params.get("t") || 0);
if (skip > 0 && match) { match.step(skip); refresh(); }
requestAnimationFrame(frame);
window.__arena = { get state() { return state; } }; // for headless checks
