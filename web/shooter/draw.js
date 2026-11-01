// Neon vector renderer: parallax starfield, glow strokes, particles, light shake. No sim state is changed here.
import { W, H, enemyColor, rng } from "./sim.js";

export function makeFx(seed = 9) {
  const r = rng(seed);
  const stars = [0.15, 0.35, 0.7].map((depth, i) => Array.from({ length: 70 - i * 18 }, () => ({ x: r() * W, y: r() * H, depth, s: 0.6 + i * 0.6 })));
  return { stars, parts: [], shake: 0, trails: new Map(), r: Math.random };
}

/** Turn sim events into particles and shake. */
export function feed(fx, w, focusIds = null) {
  for (const e of w.events) {
    if (e.k === "boom") burst(fx, e.x, e.y, e.c, e.big ? 60 : 22, e.big ? 260 : 170), (fx.shake = Math.max(fx.shake, e.big ? 12 : 3));
    else if (e.k === "spark") burst(fx, e.x, e.y, e.c, 5, 90);
    else if (e.k === "hit" && (!focusIds || focusIds.includes(e.id))) { fx.shake = Math.max(fx.shake, 7); burst(fx, e.x, e.y, "#FF5F7A", 14, 140); }
  }
}
function burst(fx, x, y, c, n, sp) {
  for (let i = 0; i < n; i++) { const a = fx.r() * 6.283, v = sp * (0.3 + fx.r()); fx.parts.push({ x, y, vx: Math.cos(a) * v, vy: Math.sin(a) * v, life: 0.4 + fx.r() * 0.5, c }); }
  if (fx.parts.length > 900) fx.parts.splice(0, fx.parts.length - 900);
}

const HULL = {
  dart: [[16, 0], [-10, -9], [-5, 0], [-10, 9]],
  delta: [[15, 0], [-9, -12], [-9, 12]],
  wing: [[14, 0], [0, -5], [-8, -14], [-5, 0], [-8, 14], [0, 5]],
};
export function drawShip(ctx, x, y, ax, ay, look, alpha = 1, pilot = false, k = 1) {
  ctx.save(); ctx.translate(x, y); ctx.scale(k, k); ctx.rotate(Math.atan2(ay, ax)); ctx.globalAlpha = alpha;
  ctx.strokeStyle = look.color; ctx.shadowColor = look.color; ctx.shadowBlur = 14; ctx.lineWidth = 2.2; ctx.lineJoin = "round";
  ctx.beginPath(); HULL[look.hull].forEach(([px, py], i) => (i ? ctx.lineTo(px, py) : ctx.moveTo(px, py))); ctx.closePath(); ctx.stroke();
  ctx.fillStyle = look.color + "33"; ctx.fill();
  if (pilot) { ctx.fillStyle = "#fff"; ctx.beginPath(); ctx.arc(1, 0, 2.6, 0, 6.283); ctx.fill(); }
  ctx.restore();
}
function poly(ctx, x, y, n, r, rot) { ctx.beginPath(); for (let i = 0; i < n; i++) { const a = rot + (i / n) * 6.283; ctx.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r); } ctx.closePath(); }

/** looks[i] = {hull,color,trail,pilot} for ship i. cam = {x,y} focus for parallax. */
export function render(ctx, w, fx, looks, dt) {
  ctx.save();
  // Phones draw the same world at ~0.4×, so sprites and strokes are drawn bigger there (looks only).
  const k = ctx.canvas.clientWidth && ctx.canvas.clientWidth < 600 ? 1.8 : 1;
  ctx.fillStyle = "#070B14"; ctx.fillRect(0, 0, W, H);
  const alive = w.ships.filter((s) => s.alive), cx = alive.length ? alive.reduce((a, s) => a + s.x, 0) / alive.length : W / 2, cy = alive.length ? alive.reduce((a, s) => a + s.y, 0) / alive.length : H / 2;
  for (const layer of fx.stars) for (const s of layer) {
    const x = (((s.x - (cx - W / 2) * s.depth * 0.5 + w.t * 8 * s.depth) % W) + W) % W, y = (((s.y - (cy - H / 2) * s.depth * 0.5) % H) + H) % H;
    ctx.fillStyle = `rgba(190,220,255,${0.25 + s.depth * 0.6})`; ctx.fillRect(x, y, s.s * k, s.s * k);
  }
  if (fx.shake > 0.1) { ctx.translate((fx.r() - 0.5) * fx.shake, (fx.r() - 0.5) * fx.shake); fx.shake *= 0.86; }
  ctx.lineWidth = 2;
  // grid glow at the edges
  ctx.strokeStyle = "#5F88B755"; ctx.strokeRect(2, 2, W - 4, H - 4);
  if (w.layout === "ring") {
    ctx.strokeStyle = "#FF5FD2"; ctx.shadowColor = "#FF5FD2"; ctx.shadowBlur = 18; ctx.lineWidth = 3;
    ctx.beginPath(); ctx.arc(W / 2, H / 2, w.ring, 0, 6.283); ctx.stroke();
    ctx.shadowBlur = 0; ctx.fillStyle = "rgba(255,95,210,.10)"; ctx.beginPath(); ctx.rect(0, 0, W, H); ctx.moveTo(W / 2 + w.ring, H / 2); ctx.arc(W / 2, H / 2, w.ring, 0, Math.PI * 2, true); ctx.fill("evenodd");
  }
  ctx.shadowBlur = 10; ctx.lineWidth = 2 * k;
  for (const k of w.rocks) { ctx.strokeStyle = "#8FA3C0"; ctx.shadowColor = "#5F88B7"; poly(ctx, k.x, k.y, 7, k.r, k.x); ctx.stroke(); }
  for (const e of w.enemies) {
    const c = enemyColor(e.type); ctx.strokeStyle = c; ctx.shadowColor = c;
    const er = e.r * (k > 1 ? 1.4 : 1), spin = w.t * 2 + e.x * 0.01;
    if (e.type === "drifter") poly(ctx, e.x, e.y, 4, er, spin * 0.5);
    else if (e.type === "chaser") poly(ctx, e.x, e.y, 3, er, Math.atan2(e.vy, e.vx));
    else if (e.type === "shooter") poly(ctx, e.x, e.y, 4, er, Math.PI / 4);
    else poly(ctx, e.x, e.y, 6, er, spin);
    ctx.stroke();
    if (e.type === "shooter") { ctx.beginPath(); ctx.arc(e.x, e.y, 4, 0, 6.283); ctx.stroke(); }
  }
  // trails
  w.ships.forEach((s, i) => {
    const look = looks[i]; let tr = fx.trails.get(i); if (!tr) fx.trails.set(i, (tr = []));
    if (s.alive) { tr.push([s.x, s.y]); if (tr.length > 18) tr.shift(); } else tr.length = 0;
    if (look.trail === "none" || tr.length < 2) return;
    ctx.strokeStyle = look.color; ctx.shadowColor = look.color; ctx.fillStyle = look.color;
    ctx.lineWidth = 2 * k;
    if (look.trail === "line") { ctx.globalAlpha = 0.45; ctx.beginPath(); tr.forEach(([x, y], j) => (j ? ctx.lineTo(x, y) : ctx.moveTo(x, y))); ctx.stroke(); }
    else tr.forEach(([x, y], j) => { if (j % 3 === 0) { ctx.globalAlpha = j / tr.length; ctx.fillRect(x - 1.5, y - 1.5, 3, 3); } });
    ctx.globalAlpha = 1;
  });
  for (const b of w.bullets) {
    const c = b.from < 0 ? "#FFC85F" : looks[b.from].color;
    ctx.strokeStyle = c; ctx.shadowColor = c; ctx.lineWidth = (b.from < 0 ? 4 : 2.5) * k;
    ctx.beginPath(); ctx.moveTo(b.x, b.y); ctx.lineTo(b.x - b.vx * 0.02, b.y - b.vy * 0.02); ctx.stroke();
  }
  ctx.lineWidth = 2;
  w.ships.forEach((s, i) => { if (s.alive && (s.inv <= 0 || Math.floor(w.t * 20) % 2)) drawShip(ctx, s.x, s.y, s.ax, s.ay, looks[i], 1, !!looks[i].pilot, k); });
  ctx.shadowBlur = 0;
  for (const p of fx.parts) { p.x += p.vx * dt; p.y += p.vy * dt; p.vx *= 0.95; p.vy *= 0.95; p.life -= dt; ctx.globalAlpha = Math.max(0, p.life * 1.6); ctx.fillStyle = p.c; ctx.fillRect(p.x - 1.5 * k, p.y - 1.5 * k, 3 * k, 3 * k); }
  fx.parts = fx.parts.filter((p) => p.life > 0);
  ctx.restore();
}
