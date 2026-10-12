// The Tank Arena scene: floor grid, obstacles, shots and the tanks as Nyborgs (or classic
// hulls), drawn from the engine's JSON state. Shared by the viewer (arena.js) and the Nyborg
// app's watch screen, so a match looks the same in both. Moved verbatim from arena.js: the
// engine is Y-up with counter-clockwise angles, the canvas Y-down, so Y flips and angles negate.
import { drawNyborg, hairColor, idleBob, facingFromHeading } from "./nyborg.js";

export const TEAM_COLORS = ["#4da3ff", "#ff6b3d"];
export const TANK_RADIUS = 16; // engine TankParams::default().radius (shared by every tank)

/**
 * Draw `state` (WasmMatch.stateJson) onto a canvas already sized state.width x state.height.
 * - `looks[team]`: that Nyborg's look ({hair_style, hair_color, strands, accessories}) or null
 *   for the team-colour default; `seeds[team]` keys its fur jitter.
 * - `facingById`: a Map kept across frames so flips stay stable near vertical headings.
 * - `sprites`: "nyborg" (default) or "classic".
 */
export function drawTankScene(ctx, state, { looks = [null, null], seeds = ["tank-0", "tank-1"], facingById = new Map(), sprites = "nyborg" } = {}) {
  const { width: W, height: H } = state;
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

  for (const t of state.tanks) drawTank(ctx, state, t, Y, { looks, seeds, facingById, sprites });
}

function drawTank(ctx, state, t, Y, opts) {
  if (opts.sprites === "classic") drawClassicTank(ctx, t, Y);
  else drawNyborgTank(ctx, state, t, Y, opts);
}

function drawNyborgTank(ctx, state, t, Y, { looks, seeds, facingById }) {
  const color = TEAM_COLORS[t.team] || "#ccc";
  const r = TANK_RADIUS;
  const prev = facingById.get(t.id) ?? 1;
  const facing = facingFromHeading(t.heading, prev);
  facingById.set(t.id, facing);
  const bob = t.alive ? idleBob(state?.tick) : 0;

  ctx.save();
  ctx.translate(t.x, Y(t.y));
  ctx.globalAlpha = t.alive ? 1 : 0.3;

  // Nyborg body (replaces the classic hull). Hair colour is team-primary unless the link
  // carries this Nyborg's look; flip on heading x. Team colour stays on the turret hub.
  // Fur jitter is keyed by the Nyborg's id when known, else the link's name or the team.
  const look = looks[t.team];
  const seed = seeds[t.team] || `tank-${t.team}`;
  drawNyborg(ctx, look
    ? { hair: look.hair_color, style: look.hair_style, id: seed, strands: look.strands, accessories: look.accessories, facing, bob, scale: 0.72 }
    : { hair: hairColor(t.team), id: seed, facing, bob, scale: 0.72 });

  // Compact aim indicator (turret barrel) so aim direction stays readable.
  ctx.save();
  ctx.rotate(-t.turret);
  ctx.fillStyle = "#e6e8ee";
  ctx.fillRect(4, -2, r + 6, 4);
  ctx.fillStyle = color;
  ctx.beginPath(); ctx.arc(0, 0, 4.5, 0, Math.PI * 2); ctx.fill();
  ctx.restore();

  drawHpOrX(ctx, t, r);
  ctx.restore();
}

function drawClassicTank(ctx, t, Y) {
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

  drawHpOrX(ctx, t, r);
  ctx.restore();
}

function drawHpOrX(ctx, t, r) {
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
