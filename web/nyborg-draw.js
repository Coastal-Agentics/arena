// Shared Nyborg drawing (canvas 2D). Used by the arena viewer (via ./nyborg.js) and the
// Customizer, so a Nyborg looks exactly the same everywhere. Cosmetic only: nothing here
// reaches the sim. The look follows docs/design/nyborg.md (revision 3): a cream head, two dot
// eyes, no mouth, and 2-4 chunky yarn strands. Drawn facing right; pass facing -1 to mirror.

export const NYBORG = {
  head: "#EADFCB",
  outline: "#5E544B",
  eyes: "#2F2A26",
  // Distinct primary yarn: tank teams use Cobalt / Yarn Red; racing uses RACE_HAIR in race-ui.js.
  hair: ["#3F6FD8", "#D9534F"],
};

/** The hair palette from nyborg.md section 2 (primary colours, no green). Yarn Red is the default. */
export const HAIR_PALETTE = [
  { hex: "#D9534F", name: "Yarn Red" },
  { hex: "#B83246", name: "Crimson" },
  { hex: "#EE7A68", name: "Coral" },
  { hex: "#3F6FD8", name: "Cobalt" },
  { hex: "#5AA9E6", name: "Sky" },
  { hex: "#2F4A7A", name: "Navy" },
  { hex: "#F2C14E", name: "Sunny" },
  { hex: "#D4A23A", name: "Mustard" },
  { hex: "#F08A3C", name: "Orange" },
  { hex: "#C8459A", name: "Magenta" },
];

/** Cosmetic upgrades defined in nyborg.md (mockup colours). */
export const ACCESSORIES = [
  { id: "glasses", name: "Glasses" },
  { id: "hat", name: "Hat" },
];

// Strand layouts by count: root x on the crown, tip x/y, and a bulge toward the curl.
// The 3-strand set is the one the viewer has drawn since #67; keep it byte-for-byte.
const STRAND_SETS = {
  2: [
    { rx: -6, tx: -21, ty: -36, bx: -15 },
    { rx: 6, tx: 21, ty: -36, bx: 15 },
  ],
  3: [
    { rx: -10, tx: -26, ty: -32, bx: -20 },
    { rx: 0, tx: -2, ty: -40, bx: 5 },
    { rx: 10, tx: 26, ty: -32, bx: 20 },
  ],
  4: [
    { rx: -13, tx: -30, ty: -27, bx: -22 },
    { rx: -4, tx: -11, ty: -39, bx: -5 },
    { rx: 4, tx: 11, ty: -39, bx: 5 },
    { rx: 13, tx: 30, ty: -27, bx: 22 },
  ],
};

/** Hair color for a tank team index; falls back to Yarn Red. */
export function hairColor(team) {
  return NYBORG.hair[team] || "#D9534F";
}

/**
 * Draw a Nyborg centered at the current transform origin.
 * - `facing`: +1 (right) or -1 (left). `bob`: small vertical offset in px.
 * - `strands`: 2, 3 or 4 (default 3). `accessories`: any of "glasses", "hat".
 * Cosmetics only — no effect on stats.
 */
export function drawNyborg(ctx, { hair, facing = 1, bob = 0, scale = 0.55, strands = 3, accessories = [] } = {}) {
  const color = hair || "#D9534F";
  const set = STRAND_SETS[strands] || STRAND_SETS[3];
  ctx.save();
  ctx.scale(facing * scale, scale);
  ctx.translate(0, bob / scale);

  // Soft ground shadow (reads as perched / standing).
  ctx.fillStyle = "rgba(0,0,0,0.18)";
  ctx.beginPath();
  ctx.ellipse(0, 8, 20, 4.5, 0, 0, Math.PI * 2);
  ctx.fill();

  // Yarn strands first so the head sits in front of the roots.
  for (const s of set) drawStrand(ctx, s, color);

  // Cream head.
  ctx.fillStyle = NYBORG.head;
  ctx.strokeStyle = NYBORG.outline;
  ctx.lineWidth = 2.6;
  ctx.beginPath();
  ctx.arc(0, -4, 20, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  // Soft highlight on the crown.
  ctx.fillStyle = "rgba(255,255,255,0.28)";
  ctx.beginPath();
  ctx.ellipse(-6, -12, 5, 2.8, -0.4, 0, Math.PI * 2);
  ctx.fill();

  // Two dot eyes (no mouth), with tiny catchlights.
  drawEye(ctx, -7, -3);
  drawEye(ctx, 7, -3);

  if (accessories.includes("glasses")) drawGlasses(ctx);
  if (accessories.includes("hat")) drawHat(ctx);

  ctx.restore();
}

function drawEye(ctx, x, y) {
  ctx.fillStyle = NYBORG.eyes;
  ctx.beginPath();
  ctx.arc(x, y, 2.6, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  ctx.arc(x + 0.8, y - 0.9, 0.8, 0, Math.PI * 2);
  ctx.fill();
}

// Glasses and hat: the nyborg-mockup.svg shapes, scaled to this head (r 20 vs the mockup's 31).
function drawGlasses(ctx) {
  ctx.save();
  ctx.strokeStyle = "#3B3632";
  ctx.lineWidth = 1.5;
  ctx.fillStyle = "rgba(255,255,255,0.25)";
  for (const x of [-7, 7]) {
    ctx.beginPath();
    ctx.arc(x, -3, 5.1, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }
  ctx.beginPath();
  ctx.moveTo(-1.9, -3.6); ctx.quadraticCurveTo(0, -5, 1.9, -3.6); // bridge
  ctx.moveTo(-12.1, -4.2); ctx.lineTo(-18.5, -6.2); // temples
  ctx.moveTo(12.1, -4.2); ctx.lineTo(18.5, -6.2);
  ctx.stroke();
  ctx.restore();
}

function drawHat(ctx) {
  ctx.save();
  ctx.translate(0, -21.5);
  ctx.rotate((8 * Math.PI) / 180);
  ctx.strokeStyle = NYBORG.outline;
  ctx.lineWidth = 1.5;
  ctx.lineJoin = "round";
  ctx.fillStyle = "#5F7A99"; // brim
  ctx.beginPath();
  ctx.ellipse(0, 0, 15.5, 3.3, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = "#6E8BAE"; // crown
  ctx.beginPath();
  ctx.moveTo(-8.4, -0.6);
  ctx.bezierCurveTo(-8.4, -8.4, -4.5, -11, 0, -11);
  ctx.bezierCurveTo(4.5, -11, 8.4, -8.4, 8.4, -0.6);
  ctx.closePath();
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = "#F5F2EC"; // band
  ctx.fillRect(-8, -4.2, 16, 2.8);
  ctx.restore();
}

function drawStrand(ctx, s, color) {
  const rootY = -18;
  const tipY = rootY + s.ty;
  const midX = (s.rx + s.tx) / 2 + s.bx * 0.15;
  const midY = rootY + s.ty * 0.55;

  // Outline (chunky yarn silhouette).
  strokeCurve(ctx, s.rx, rootY, midX, midY, s.tx, tipY, NYBORG.outline, 11, "round");
  // Fill colour.
  strokeCurve(ctx, s.rx, rootY, midX, midY, s.tx, tipY, color, 7.5, "round");
  // Ply ticks (twisted yarn feel).
  ctx.save();
  ctx.strokeStyle = "rgba(0,0,0,0.18)";
  ctx.lineWidth = 1.1;
  ctx.lineCap = "round";
  for (let i = 1; i <= 5; i++) {
    const t = i / 6;
    const [px, py] = quadPoint(s.rx, rootY, midX, midY, s.tx, tipY, t);
    const [tx, ty] = quadTangent(s.rx, rootY, midX, midY, s.tx, tipY, t);
    const len = Math.hypot(tx, ty) || 1;
    const nx = -ty / len, ny = tx / len;
    ctx.beginPath();
    ctx.moveTo(px - nx * 2.2, py - ny * 2.2);
    ctx.lineTo(px + nx * 2.2, py + ny * 2.2);
    ctx.stroke();
  }
  ctx.restore();
  // Soft highlight along one side.
  strokeCurve(
    ctx,
    s.rx - 1.2, rootY + 1,
    midX - 1.5, midY,
    s.tx - 1.2, tipY + 1,
    "rgba(255,255,255,0.32)", 1.3, "round",
  );
}

function strokeCurve(ctx, x0, y0, x1, y1, x2, y2, stroke, width, cap) {
  ctx.beginPath();
  ctx.moveTo(x0, y0);
  ctx.quadraticCurveTo(x1, y1, x2, y2);
  ctx.strokeStyle = stroke;
  ctx.lineWidth = width;
  ctx.lineCap = cap;
  ctx.lineJoin = "round";
  ctx.stroke();
}

function quadPoint(x0, y0, x1, y1, x2, y2, t) {
  const u = 1 - t;
  return [u * u * x0 + 2 * u * t * x1 + t * t * x2, u * u * y0 + 2 * u * t * y1 + t * t * y2];
}

function quadTangent(x0, y0, x1, y1, x2, y2, t) {
  return [2 * (1 - t) * (x1 - x0) + 2 * t * (x2 - x1), 2 * (1 - t) * (y1 - y0) + 2 * t * (y2 - y1)];
}

/** Deterministic idle bob in px from a sim tick (cosmetic only). */
export function idleBob(tick) {
  return Math.sin((tick || 0) * 0.12) * 1.6;
}

/**
 * Facing from heading's x component (cos). Sprites face right by default;
 * negative x flips left. `prev` (+1/-1) is kept inside a small deadzone so the
 * sprite doesn't flicker when the hull points nearly straight up or down.
 */
export function facingFromHeading(heading, prev = 1) {
  const x = Math.cos(heading);
  if (Math.abs(x) < 0.12) return prev;
  return x >= 0 ? 1 : -1;
}
