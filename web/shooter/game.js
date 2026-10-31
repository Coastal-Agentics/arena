// Shooter page: menu, Ship Customizer, game loop + input (keys, mouse, gamepad, touch), Train.
import { W, H, DT, BUDGET, STAT_MAX, STATS, HULLS, COLORS, TRAILS, GEN0, createWorld, step } from "./sim.js";
import { makeFx, feed, render, drawShip } from "./draw.js";
import { loadShip, saveShip, cleanShip, left, pilots } from "./ships.js";

const $ = (s) => document.querySelector(s);
const store = (() => { try { localStorage.setItem("arena.shooter.probe", "1"); localStorage.removeItem("arena.shooter.probe"); return localStorage; } catch { const m = new Map(); return { getItem: (k) => m.get(k) ?? null, setItem: (k, v) => m.set(k, v), removeItem: (k) => m.delete(k) }; } })();
const BRAIN_KEY = "arena.shooter.brain";
const AI_LOOK = { hull: "delta", color: "#FF5FD2", trail: "line", pilot: null };
const GEN0_LOOK = { hull: "dart", color: "#A98BFF", trail: "dots", pilot: null };
let ship = loadShip(store), champion = { genes: GEN0 }, shipped = null;
const brain = () => { try { const b = JSON.parse(store.getItem(BRAIN_KEY) || "null"); if (b?.genes) return b.genes; } catch {} return champion.genes; };

fetch("champion.json?v=20261029a").then((r) => r.json()).then((c) => { champion = shipped = c; menuLine(); }).catch(() => {});

// ---------- screens ----------
const params = new URLSearchParams(location.hash.slice(1));
function show(id) { for (const s of document.querySelectorAll(".sh-screen")) s.hidden = s.id !== id; if (id !== "game") stop(); }
document.addEventListener("click", (e) => { if (e.target.closest("[data-back]")) { show("menu"); menuLine(); } });
function menuLine() {
  const p = pilots(store).find((n) => n.id === ship.pilot);
  const c = shipped?.champion?.overall, g = shipped?.gen0?.overall;
  $("#ship-line").textContent = `Your ship: ${ship.name} · speed ${ship.build.speed} · fire ${ship.build.fire} · damage ${ship.build.damage} · shield ${ship.build.shield}${p ? ` · pilot ${p.name}` : ""}` +
    (c && g ? `. Champion vs Gen 0 on 10 held-out seeds: ${Math.round(c.score)} vs ${Math.round(g.score)} points a run.` : "");
}

// ---------- customizer ----------
let draft;
function openCustom() {
  draft = cleanShip(ship); show("custom");
  $("#ship-name").value = draft.name;
  const sel = $("#pilot"); sel.length = 1;
  for (const n of pilots(store)) sel.add(new Option(n.name, n.id));
  sel.value = draft.pilot || "";
  paintCustom();
}
function seg(el, items, cur, on) {
  el.replaceChildren(...items.map((v) => { const b = document.createElement("button"); b.type = "button"; b.textContent = v; b.setAttribute("aria-pressed", v === cur); b.onclick = () => on(v); return b; }));
}
function paintCustom() {
  const stats = $("#stats"); stats.replaceChildren();
  for (const k of STATS) {
    const row = document.createElement("div"); row.className = "sh-stat";
    const minus = Object.assign(document.createElement("button"), { type: "button", textContent: "−", ariaLabel: `less ${k}` });
    const plus = Object.assign(document.createElement("button"), { type: "button", textContent: "+", ariaLabel: `more ${k}` });
    minus.disabled = draft.build[k] <= 0; plus.disabled = draft.build[k] >= STAT_MAX || left(draft.build) <= 0;
    minus.onclick = () => { draft.build[k]--; paintCustom(); }; plus.onclick = () => { draft.build[k]++; paintCustom(); };
    const pips = document.createElement("div"); pips.className = "sh-pips";
    for (let i = 0; i < STAT_MAX; i++) pips.append(Object.assign(document.createElement("i"), { className: i < draft.build[k] ? "on" : "" }));
    const lab = document.createElement("span"); lab.textContent = { speed: "Speed", fire: "Fire rate", damage: "Damage", shield: "Shield" }[k];
    row.append(lab, minus, pips, plus); stats.append(row);
  }
  const l = left(draft.build);
  $("#budget").textContent = l ? `${l} of ${BUDGET} points left. Spend them all to save.` : `All ${BUDGET} points spent.`;
  $("#save-ship").disabled = l !== 0;
  seg($("#hulls"), HULLS, draft.look.hull, (v) => { draft.look.hull = v; paintCustom(); });
  seg($("#trails"), TRAILS, draft.look.trail, (v) => { draft.look.trail = v; paintCustom(); });
  $("#colors").replaceChildren(...COLORS.map((c) => { const b = document.createElement("button"); b.type = "button"; b.className = "sh-swatch"; b.style.background = c; b.ariaLabel = c; b.setAttribute("aria-pressed", c === draft.look.color); b.onclick = () => { draft.look.color = c; paintCustom(); }; return b; }));
  const cv = $("#preview"), ctx = cv.getContext("2d");
  ctx.fillStyle = "#070B14"; ctx.fillRect(0, 0, cv.width, cv.height);
  ctx.save(); ctx.translate(cv.width / 2, cv.height / 2); ctx.scale(4, 4);
  if (draft.look.trail !== "none") { ctx.strokeStyle = draft.look.color; ctx.fillStyle = draft.look.color; ctx.globalAlpha = 0.5; ctx.lineWidth = 0.8;
    if (draft.look.trail === "line") { ctx.beginPath(); ctx.moveTo(-10, 0); ctx.lineTo(-34, 0); ctx.stroke(); } else for (let x = -12; x > -36; x -= 5) ctx.fillRect(x, -0.6, 1.2, 1.2); ctx.globalAlpha = 1; }
  drawShip(ctx, 0, 0, 1, 0, draft.look, 1, !!$("#pilot").value);
  ctx.restore();
}
$("#ship-name").oninput = (e) => (draft.name = e.target.value);
$("#pilot").onchange = (e) => { draft.pilot = e.target.value || null; paintCustom(); };
$("#save-ship").onclick = () => { ship = saveShip(store, draft); show("menu"); menuLine(); };
$("#go-custom").onclick = openCustom;

// ---------- input ----------
const keys = new Set(); const mouse = { x: W / 2, y: H / 2, down: false, active: false };
addEventListener("keydown", (e) => { keys.add(e.key.toLowerCase()); if ((e.key === "p" || e.key === "Escape") && run) togglePause(); });
addEventListener("keyup", (e) => keys.delete(e.key.toLowerCase()));
const touch = { move: null, aim: null };
function bindStage(cv) {
  const pos = (e) => { const r = cv.getBoundingClientRect(); return { x: ((e.clientX - r.left) / r.width) * W, y: ((e.clientY - r.top) / r.height) * H }; };
  cv.addEventListener("pointermove", (e) => { if (e.pointerType === "mouse") Object.assign(mouse, pos(e), { active: true }); else moveTouch(e, pos(e)); });
  cv.addEventListener("pointerdown", (e) => { if (e.pointerType === "mouse") { mouse.down = true; Object.assign(mouse, pos(e), { active: true }); } else { cv.setPointerCapture(e.pointerId); const p = pos(e); const side = p.x < W / 2 ? "move" : "aim"; touch[side] = { id: e.pointerId, ox: p.x, oy: p.y, x: p.x, y: p.y }; } e.preventDefault(); });
  const up = (e) => { if (e.pointerType === "mouse") mouse.down = false; for (const k of ["move", "aim"]) if (touch[k]?.id === e.pointerId) touch[k] = null; };
  cv.addEventListener("pointerup", up); cv.addEventListener("pointercancel", up);
  cv.addEventListener("contextmenu", (e) => e.preventDefault());
}
function moveTouch(e, p) { for (const k of ["move", "aim"]) if (touch[k]?.id === e.pointerId) Object.assign(touch[k], p); }
function humanInput(s) {
  let mx = (keys.has("d") || keys.has("arrowright") ? 1 : 0) - (keys.has("a") || keys.has("arrowleft") ? 1 : 0);
  let my = (keys.has("s") || keys.has("arrowdown") ? 1 : 0) - (keys.has("w") || keys.has("arrowup") ? 1 : 0);
  let ax = s.ax, ay = s.ay, fire = false;
  if (mouse.active) { ax = mouse.x - s.x; ay = mouse.y - s.y; fire = mouse.down || keys.has(" "); }
  if (touch.move) { mx = (touch.move.x - touch.move.ox) / 60; my = (touch.move.y - touch.move.oy) / 60; }
  if (touch.aim) { const dx = touch.aim.x - touch.aim.ox, dy = touch.aim.y - touch.aim.oy; if (Math.hypot(dx, dy) > 8) { ax = dx; ay = dy; } fire = true; }
  for (const gp of navigator.getGamepads?.() || []) {
    if (!gp) continue; const dz = (v) => (Math.abs(v) < 0.18 ? 0 : v);
    if (dz(gp.axes[0]) || dz(gp.axes[1])) { mx = dz(gp.axes[0]); my = dz(gp.axes[1]); }
    const rx = dz(gp.axes[2] ?? 0), ry = dz(gp.axes[3] ?? 0);
    if (Math.hypot(rx, ry) > 0.3) { ax = rx; ay = ry; fire = true; mouse.active = false; }
    if (gp.buttons[7]?.pressed || gp.buttons[0]?.pressed) fire = true;
  }
  return { mx, my, ax, ay, fire };
}
function drawSticks(ctx) {
  ctx.save(); ctx.lineWidth = 2;
  for (const [k, c] of [["move", "#5FE1FF"], ["aim", "#FF5FD2"]]) { const t = touch[k]; if (!t) continue;
    ctx.strokeStyle = c + "88"; ctx.beginPath(); ctx.arc(t.ox, t.oy, 60, 0, 6.283); ctx.stroke();
    ctx.fillStyle = c + "66"; ctx.beginPath(); const dx = t.x - t.ox, dy = t.y - t.oy, d = Math.min(60, Math.hypot(dx, dy)) / (Math.hypot(dx, dy) || 1); ctx.arc(t.ox + dx * d, t.oy + dy * d, 22, 0, 6.283); ctx.fill(); }
  ctx.restore();
}

// ---------- game ----------
let run = null;
function stage(label) {
  const div = document.createElement("div"); div.className = "sh-stage";
  div.innerHTML = `<canvas width="${W}" height="${H}"></canvas><div class="sh-hud"><span class="l"></span><span class="r"></span></div><div class="sh-over" hidden><h2></h2><p></p><div class="sh-row" style="justify-content:center"><button class="sh-btn primary" data-again>Again · same seed</button><button class="sh-btn" data-new>New seed</button></div></div>${label ? `<span class="tag">${label}</span>` : ""}`;
  return div;
}
function start(mode) {
  const layout = $("#layout").value, seed = Math.max(1, parseInt($("#seed").value, 10) || 1);
  const me = { build: ship.build, ai: null }, bot = { build: ship.build, ai: brain() };
  const lookMe = { ...ship.look, pilot: ship.pilot };
  const sims = {
    play: [{ pilots: [me], looks: [lookMe] }],
    watch: [{ pilots: [bot], looks: [{ ...AI_LOOK }], label: "Champion" }],
    compare: [{ pilots: [{ build: ship.build, ai: GEN0 }], looks: [GEN0_LOOK], label: "Gen 0" }, { pilots: [bot], looks: [{ ...AI_LOOK }], label: "Champion" }],
    versus: [{ pilots: [me, bot], looks: [lookMe, { ...AI_LOOK }] }],
    coop: [{ pilots: [me, bot], looks: [lookMe, { ...AI_LOOK }] }],
  }[mode];
  stop(); show("game");
  const host = $("#stages"); host.replaceChildren(); host.className = sims.length > 1 ? "sh-pair" : "";
  run = { mode, layout, seed, paused: false, acc: 0, last: performance.now(), views: sims.map((s) => {
    const el = stage(s.label); host.append(el); bindStage(el.querySelector("canvas"));
    el.querySelector("[data-again]").onclick = () => start(mode);
    el.querySelector("[data-new]").onclick = () => { $("#seed").value = 1 + Math.floor(Math.random() * 99998); start(mode); };
    return { el, ctx: el.querySelector("canvas").getContext("2d"), w: createWorld({ seed, layout, pilots: s.pilots, maxTime: 600 }), fx: makeFx(seed), looks: s.looks, label: s.label };
  }) };
  $("#game-note").textContent = { play: matchMedia("(pointer: coarse)").matches ? "Left thumb moves, right thumb aims and fires." : "", watch: "The champion, trained offline with the same sim.", compare: "Same seed and ship for both. Only the behavior genes differ.", versus: "You (your color) vs the champion (pink). Higher score wins.", coop: "You and the champion share the waves and the score." }[mode];
  $("#pause").textContent = "Pause";
  if (innerWidth < 700) host.scrollIntoView({ block: "start" });
  run.raf = requestAnimationFrame(loop);
}
function stop() { if (run) cancelAnimationFrame(run.raf); run = null; touch.move = touch.aim = null; }
function togglePause() { run.paused = !run.paused; $("#pause").textContent = run.paused ? "Resume" : "Pause"; }
$("#pause").onclick = () => run && togglePause();
function loop(now) {
  const r = run; if (!r) return;
  const dt = Math.min(0.05, (now - r.last) / 1000); r.last = now;
  if (!r.paused) r.acc += dt;
  while (r.acc >= DT) {
    r.acc -= DT;
    for (const v of r.views) if (!v.w.over) { const hs = v.w.ships.find((s) => !s.ai); step(v.w, hs ? { [hs.id]: humanInput(hs) } : []); feed(v.fx, v.w, v.w.ships.filter((s) => !s.ai).map((s) => s.id)); }
  }
  for (const v of r.views) {
    render(v.ctx, v.w, v.fx, v.looks, r.paused ? 0 : dt); drawSticks(v.ctx); hud(r, v);
  }
  r.raf = requestAnimationFrame(loop);
}
function hud(r, v) {
  const w = v.w, [a, b] = w.ships, hp = (s) => "▮".repeat(Math.max(0, s.hp)) + "▯".repeat(Math.max(0, s.maxHp - Math.max(0, s.hp)));
  const L = v.el.querySelector(".l"), R = v.el.querySelector(".r");
  L.textContent = r.mode === "versus" ? `You ${a.score} ${hp(a)}  ·  AI ${b.score} ${hp(b)}` : `${w.score}  ${hp(a)}${b ? "  " + hp(b) : ""}`;
  R.textContent = `Wave ${w.wave} · ${Math.floor(w.t)}s${r.paused ? " · paused" : ""}`;
  const over = v.el.querySelector(".sh-over");
  if (w.over && over.hidden) {
    over.hidden = false;
    over.querySelector("h2").textContent = r.mode === "versus" ? (a.score > b.score ? "You win" : a.score < b.score ? "The AI wins" : "Tie") : "Run over";
    over.querySelector("p").textContent = `${w.score} points · wave ${w.wave} · ${Math.floor(w.t)} s · seed ${r.seed}`;
  }
}
for (const b of document.querySelectorAll("[data-mode]")) b.onclick = () => start(b.dataset.mode);

// ---------- train ----------
let worker = null, hist = [], result = null;
$("#go-train").onclick = () => { show("train"); chart(); };
$("#train-start").onclick = () => {
  worker?.terminate(); hist = []; result = null; $("#train-use").hidden = true;
  const gens = +$("#gens").value;
  worker = new Worker(new URL("./worker.js?v=20261029a", import.meta.url), { type: "module" });
  $("#train-status").textContent = "Generation 1 is running…";
  worker.onerror = (e) => { $("#train-status").textContent = `Training stopped: ${e.message || "the worker failed to start"}.`; };
  worker.postMessage({ seed: 1 + Math.floor(Math.random() * 9999), pop: 16, gens });
  worker.onmessage = ({ data }) => {
    if (data.type === "gen") { hist.push(data); $("#train-status").textContent = `Generation ${data.gen + 1} of ${data.gens} · best fitness ${data.best.toFixed(0)} · mean ${data.mean.toFixed(0)}`; chart(); }
    else { result = data; $("#train-status").textContent = `Done. On 3 held-out seeds: your brain scores ${Math.round(data.champion.score)} a run vs Gen 0's ${Math.round(data.gen0.score)} (survives ${data.champion.time.toFixed(0)} s vs ${data.gen0.time.toFixed(0)} s).`; $("#train-use").hidden = false; worker.terminate(); worker = null; }
  };
};
$("#train-use").onclick = () => { store.setItem(BRAIN_KEY, JSON.stringify({ genes: result.genes })); $("#train-status").textContent += " Watch AI now uses this brain."; };
$("#train-reset").onclick = () => { store.removeItem(BRAIN_KEY); $("#train-status").textContent = "Using the shipped champion."; };
function chart() {
  const cv = $("#chart"), c = cv.getContext("2d"); c.fillStyle = "#070B14"; c.fillRect(0, 0, cv.width, cv.height);
  if (!hist.length) return;
  const all = hist.flatMap((h) => [h.best, h.mean]), lo = Math.min(...all) * 0.97, hi = Math.max(...all) * 1.01, n = Math.max(2, +$("#gens").value);
  for (const [k, col] of [["mean", "#5F88B7"], ["best", "#5FE1FF"]]) { c.strokeStyle = col; c.shadowColor = col; c.shadowBlur = 8; c.lineWidth = 2.5; c.beginPath();
    hist.forEach((h, i) => { const x = 20 + (i / (n - 1)) * (cv.width - 40), y = cv.height - 15 - ((h[k] - lo) / (hi - lo || 1)) * (cv.height - 30); i ? c.lineTo(x, y) : c.moveTo(x, y); }); c.stroke(); }
}

// ---------- boot ----------
menuLine();
if (params.get("layout")) $("#layout").value = params.get("layout");
if (params.get("seed")) $("#seed").value = params.get("seed");
if (params.get("screen") === "custom") openCustom();
else if (params.get("screen") === "train") $("#go-train").click();
else if (params.get("mode")) start(params.get("mode"));
