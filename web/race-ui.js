// Racing viewer helpers (cosmetic drawing + default Ring builds). Sim stays in WasmRace.

import { drawNyborg, facingFromHeading, idleBob } from "./nyborg.js";

/** Distinct primary yarn per car (no green): Yarn Red, Cobalt, Sunny, Orange. */
export const RACE_HAIR = ["#D9534F", "#3F6FD8", "#F2C14E", "#F08A3C"];

export const RACE_BEHAVIORS = ["follower", "cutter", "blocker", "follower"];

export function hairForCar(id) {
  return RACE_HAIR[id % RACE_HAIR.length];
}

/** Default 4-car Ring grid: follower, cutter, blocker, follower — all 3/3/3. */
export function defaultRaceBuilds() {
  return RACE_BEHAVIORS.map((id) => ({
    rules_version: 1,
    levels: { power: 3, top_speed: 3, grip: 3 },
    behavior: { kind: "scripted", id },
  }));
}

export function raceBuild(id, levels = { power: 3, top_speed: 3, grip: 3 }) {
  return { rules_version: 1, levels, behavior: { kind: "scripted", id } };
}

/** Parse racing URL bits. Tank links without game=racing stay tank. */
export function raceSpecFromQuery(search) {
  const q = new URLSearchParams(typeof search === "string" ? search.replace(/^\?/, "") : search);
  const seed = (q.get("seed") || "42").trim() || "42";
  return { game: "racing", seed, builds: defaultRaceBuilds() };
}

export function queryFromRaceSpec(spec, extras = {}) {
  const parts = [`game=racing`, `seed=${encodeURIComponent(spec.seed)}`];
  if (extras.speed && extras.speed !== 1) parts.push(`speed=${extras.speed}`);
  if (extras.paused) parts.push("paused=1");
  if (extras.t) parts.push(`t=${extras.t}`);
  return parts.join("&");
}

function Yof(H, y) { return H - y; }

function pathClosed(ctx, pts, Y) {
  if (!pts.length) return;
  ctx.moveTo(pts[0][0], Y(pts[0][1]));
  for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], Y(pts[i][1]));
  ctx.closePath();
}

/** Draw the Ring: asphalt between outer/inner, faint gates, checkered start/finish. */
export function drawTrack(ctx, track) {
  const H = track.height;
  const Y = (y) => Yof(H, y);

  ctx.fillStyle = "#0e1219";
  ctx.fillRect(0, 0, track.width, H);

  // Soft grid
  ctx.strokeStyle = "#161c27";
  ctx.lineWidth = 1;
  for (let x = 50; x < track.width; x += 50) {
    ctx.beginPath(); ctx.moveTo(x + 0.5, 0); ctx.lineTo(x + 0.5, H); ctx.stroke();
  }
  for (let y = 50; y < H; y += 50) {
    ctx.beginPath(); ctx.moveTo(0, y + 0.5); ctx.lineTo(track.width, y + 0.5); ctx.stroke();
  }

  // Road (even-odd: outer ring minus inner hole).
  ctx.beginPath();
  pathClosed(ctx, track.outer, Y);
  pathClosed(ctx, track.inner, Y);
  ctx.fillStyle = "#2a3140";
  ctx.fill("evenodd");
  ctx.strokeStyle = "#3a4560";
  ctx.lineWidth = 2;
  ctx.beginPath(); pathClosed(ctx, track.outer, Y); ctx.stroke();
  ctx.beginPath(); pathClosed(ctx, track.inner, Y); ctx.stroke();

  // Faint gate ticks.
  ctx.strokeStyle = "rgba(230,232,238,0.18)";
  ctx.lineWidth = 1;
  for (const g of track.gates) {
    if (g.start_finish) continue;
    ctx.beginPath();
    ctx.moveTo(g.outer[0], Y(g.outer[1]));
    ctx.lineTo(g.inner[0], Y(g.inner[1]));
    ctx.stroke();
  }

  // Checkered start/finish along gate 0.
  const sf = track.gates[track.start_finish_gate] || track.gates[0];
  if (sf) drawCheckeredLine(ctx, sf.outer, sf.inner, Y, 10);
}

function drawCheckeredLine(ctx, a, b, Y, n) {
  const ax = a[0], ay = Y(a[1]), bx = b[0], by = Y(b[1]);
  for (let i = 0; i < n; i++) {
    const t0 = i / n, t1 = (i + 1) / n;
    const x0 = ax + (bx - ax) * t0, y0 = ay + (by - ay) * t0;
    const x1 = ax + (bx - ax) * t1, y1 = ay + (by - ay) * t1;
    // Perpendicular half-width for a fat checkered bar.
    const dx = bx - ax, dy = by - ay;
    const len = Math.hypot(dx, dy) || 1;
    const nx = (-dy / len) * 5, ny = (dx / len) * 5;
    ctx.fillStyle = i % 2 === 0 ? "#e6e8ee" : "#1a1e28";
    ctx.beginPath();
    ctx.moveTo(x0 - nx, y0 - ny);
    ctx.lineTo(x1 - nx, y1 - ny);
    ctx.lineTo(x1 + nx, y1 + ny);
    ctx.lineTo(x0 + nx, y0 + ny);
    ctx.closePath();
    ctx.fill();
  }
}

/**
 * Draw one car: kart rotated to heading, Nyborg perched and flipped on heading x.
 * Finished cars draw faded (ghosts).
 */
export function drawRaceCar(ctx, track, car, { facingPrev = 1, tick = 0 } = {}) {
  const H = track.height;
  const Y = (y) => Yof(H, y);
  const hair = hairForCar(car.id);
  const facing = facingFromHeading(car.heading, facingPrev);
  const bob = car.finished ? 0 : idleBob(tick + car.id * 7);
  const ghost = car.finished;

  ctx.save();
  ctx.translate(car.pos.x, Y(car.pos.y));
  ctx.globalAlpha = ghost ? 0.35 : 1;

  // Kart body (world-rotated with heading; canvas angles are clockwise, so negate).
  ctx.save();
  ctx.rotate(-car.heading);
  drawKart(ctx, hair);
  ctx.restore();

  // Nyborg sits on the kart; flips left/right, does not spin with the chassis.
  drawNyborg(ctx, { hair, facing, bob, scale: 0.42 });

  // Tiny place badge.
  if (car.placing != null) {
    ctx.fillStyle = ghost ? "#aaa" : "#e6e8ee";
    ctx.font = "bold 11px system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(String(car.placing), 0, 22);
  }

  ctx.restore();
  return facing;
}

function drawKart(ctx, accent) {
  // Chassis
  ctx.fillStyle = "#1a1e28";
  ctx.strokeStyle = "#0b0d12";
  ctx.lineWidth = 1.5;
  roundRect(ctx, -14, -8, 28, 16, 3);
  ctx.fill();
  ctx.stroke();
  // Nose
  ctx.fillStyle = accent;
  ctx.fillRect(8, -5, 6, 10);
  // Wheels
  ctx.fillStyle = "#0b0d12";
  ctx.fillRect(-12, -11, 7, 4);
  ctx.fillRect(-12, 7, 7, 4);
  ctx.fillRect(5, -11, 7, 4);
  ctx.fillRect(5, 7, 7, 4);
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** Finish banner overlay. */
export function drawFinishBanner(ctx, track, setup, outcome) {
  if (!outcome) return;
  const W = track.width, H = track.height;
  ctx.save();
  ctx.fillStyle = "rgba(10,12,18,0.55)";
  ctx.fillRect(0, 0, W, H);
  const boxW = 360, boxH = 40 + (outcome.placings?.length || 0) * 28 + 24;
  const bx = (W - boxW) / 2, by = (H - boxH) / 2;
  ctx.fillStyle = "#141a26";
  ctx.strokeStyle = "#3a4560";
  ctx.lineWidth = 2;
  roundRect(ctx, bx, by, boxW, boxH, 10);
  ctx.fill();
  ctx.stroke();

  ctx.fillStyle = "#e6e8ee";
  ctx.font = "bold 18px system-ui, sans-serif";
  ctx.textAlign = "center";
  const title = outcome.winner == null
    ? "Race over — no finisher"
    : `${setup?.cars?.[outcome.winner]?.name || "Car " + outcome.winner} wins!`;
  ctx.fillText(title, W / 2, by + 28);
  ctx.font = "13px system-ui, sans-serif";
  ctx.fillStyle = "#9aa3b5";
  ctx.fillText(`${(outcome.ticks / 60).toFixed(1)}s · ${outcome.reason}`, W / 2, by + 48);

  const order = [...(outcome.placings || [])]
    .map((place, id) => ({ id, place, tick: outcome.finish_ticks?.[id] }))
    .sort((a, b) => a.place - b.place);
  order.forEach((row, i) => {
    const y = by + 78 + i * 28;
    const hair = hairForCar(row.id);
    const name = setup?.cars?.[row.id]?.name || `Car ${row.id}`;
    const beh = setup?.cars?.[row.id]?.behavior || "";
    ctx.fillStyle = hair;
    ctx.beginPath(); ctx.arc(bx + 28, y - 4, 7, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "#e6e8ee";
    ctx.textAlign = "left";
    ctx.font = "bold 14px system-ui, sans-serif";
    ctx.fillText(`${row.place}. ${name}`, bx + 44, y);
    ctx.fillStyle = "#9aa3b5";
    ctx.font = "12px system-ui, sans-serif";
    const t = row.tick != null ? `${(row.tick / 60).toFixed(2)}s` : "—";
    ctx.textAlign = "right";
    ctx.fillText(`${beh} · ${t}`, bx + boxW - 20, y);
  });
  ctx.restore();
}
