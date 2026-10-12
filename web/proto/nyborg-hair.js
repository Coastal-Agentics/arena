// PROTOTYPE, not wired in: four fuzzy hair treatments for Nyborgs (docs/design/nyborg-app.md §8).
// Nothing in the viewer or the Customizer imports this yet; web/proto/hair-sheet.html draws the
// comparison sheet. Once Nye picks, the chosen style moves into nyborg-draw.js behind a
// `hair_style` look field (save format 2) and the yarn strands stay as the "classic" option.
//
// Same head as nyborg-draw.js: cream circle r 20 at (0, -4), two dot eyes at y -3, no mouth.
// Drawn facing right; `facing: -1` mirrors the whole Nyborg, so any sweep in the hair flips too.
// Deterministic: no Math.random. Jitter comes from mulberry32 seeded by an FNV-1a hash of the
// Nyborg id, so the same Nyborg always gets the same tufts, on every machine.
//
// How the fur is drawn: every tuft is a tapered, slightly curled leaf, filled (never a stroked
// line, which reads as sticks). Pass 1 draws all tufts a little larger in a dark tint of the hair
// colour, pass 2 a soft base mass, pass 3 the tufts in the hair colour; only the outer edge keeps
// the dark rim, so the silhouette reads as a fuzzy outline. At Customizer size a few thin lighter
// and darker tufts add texture; at viewer size (head under ~26 px) those are skipped and the tufts
// get fatter, so the shape still reads as one shaggy blob.
import { NYBORG, drawNyborg } from "../nyborg-draw.js";

export const HAIR_OPTIONS = [
  { id: "mop", letter: "A", name: "Shaggy mop", blurb: "A big soft mop of short fur, hanging over the sides with a messy fringe." },
  { id: "pom", letter: "B", name: "Pom cloud", blurb: "A round fluffy pom-pom cloud sitting on the crown." },
  { id: "fringe", letter: "C", name: "Muppet fringe", blurb: "A fuzzy bowl with heavy bangs falling to just above the eyes." },
  { id: "crest", letter: "D", name: "Tousled crest", blurb: "A tousled crest of soft fur tufts, swept back as if running." },
];

/** FNV-1a 32-bit hash of a string (the Nyborg id). */
export function seedFrom(id) {
  let h = 0x811c9dc5;
  for (const ch of String(id ?? "nyborg")) {
    h ^= ch.codePointAt(0);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

/** mulberry32: a tiny deterministic PRNG returning [0, 1). */
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// --- colour helpers ---------------------------------------------------------------------------
function rgb(hex) {
  const n = parseInt(hex.replace("#", ""), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
function mix(hex, to, t) {
  const a = rgb(hex), b = rgb(to);
  const c = a.map((v, i) => Math.round(v + (b[i] - v) * t));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}
/** Tones for one hair colour: a dark rim, the base, a lighter top and a darker texture. */
function tones(hex) {
  return {
    rim: mix(hex, "#2A221E", 0.58),
    base: hex,
    light: mix(hex, "#FFFFFF", 0.38),
    dark: mix(hex, "#2A221E", 0.28),
  };
}

// --- the tuft primitive -----------------------------------------------------------------------
/**
 * A filled fur clump from (x, y) toward angle `a`: `len` long, `w` wide at the root, bent by
 * `curl`, ending in a ROUND tip (radius w * tip). Round tips are what make it read as soft fur;
 * pointed tips read as spikes.
 */
function clump(ctx, x, y, a, len, w, curl, tip = 0.34) {
  const dx = Math.cos(a), dy = Math.sin(a);
  const nx = -dy, ny = dx;
  const ta = a + curl;
  const tr = Math.max(0.35, w * tip);
  const tx = x + Math.cos(ta) * (len - tr), ty = y + Math.sin(ta) * (len - tr);
  const tnx = -Math.sin(ta), tny = Math.cos(ta);
  const mx = x + dx * len * 0.5 + nx * curl * len * 0.35;
  const my = y + dy * len * 0.5 + ny * curl * len * 0.35;
  const hw = w / 2;
  const mw = (hw + tr) / 2;
  ctx.beginPath();
  ctx.moveTo(x + nx * hw, y + ny * hw);
  ctx.quadraticCurveTo(mx + nx * mw, my + ny * mw, tx + tnx * tr, ty + tny * tr);
  ctx.arc(tx, ty, tr, ta + Math.PI / 2, ta - Math.PI / 2, true);
  ctx.quadraticCurveTo(mx - nx * mw, my - ny * mw, x - nx * hw, y - ny * hw);
  ctx.arc(x, y, hw, a - Math.PI / 2, a + Math.PI / 2, true);
  ctx.closePath();
  ctx.fill();
}

/** Where a clump ends, and which way it points there. */
function tipOf(f) {
  const ta = f.a + f.curl;
  return [f.x + Math.cos(ta) * f.len, f.y + Math.sin(ta) * f.len, ta];
}

/**
 * Draw one layer of fur: clumps [{x, y, a, len, w, curl, hi}] plus an optional base mass path.
 * `small` = viewer size: fatter clumps, no fuzz or texture.
 */
function furLayer(ctx, tufts, massPath, t, small, r, extra) {
  const grow = small ? 1.3 : 1;
  // Pass 1: the dark rim. Every clump a bit bigger, plus (large only) a few micro-hairs past
  // each tip, so the outer edge looks felted and fuzzy instead of cut out.
  ctx.fillStyle = t.rim;
  for (const f of tufts) clump(ctx, f.x, f.y, f.a, f.len + 1.3, f.w * grow + 2.3, f.curl);
  if (!small) {
    for (const f of tufts) {
      const [tx, ty, ta] = tipOf(f);
      for (let k = 0; k < 3; k++) {
        const s = ta + (k - 1) * 0.55 + (r() - 0.5) * 0.4;
        clump(ctx, tx - Math.cos(ta) * f.w * 0.3, ty - Math.sin(ta) * f.w * 0.3, s, 2.4 + r() * 1.6, 1.25, (r() - 0.5) * 0.8, 0.45);
      }
    }
  }
  if (massPath) {
    ctx.save();
    ctx.lineWidth = 2.4;
    ctx.strokeStyle = t.rim;
    massPath(ctx); ctx.stroke();
    ctx.restore();
  }
  // Pass 2: base mass, so no head shows through the gaps.
  ctx.fillStyle = t.base;
  if (massPath) { massPath(ctx); ctx.fill(); }
  // Pass 3: the clumps in the hair colour.
  for (const f of tufts) clump(ctx, f.x, f.y, f.a, f.len, f.w * grow, f.curl);
  if (extra) extra(ctx, t, small);
  if (small) return;
  // Texture: soft darker strands inside, lighter strands on the side the light comes from.
  ctx.fillStyle = t.dark;
  for (const f of tufts) {
    if (r() < 0.4) clump(ctx, f.x + (r() - 0.5) * 2, f.y + (r() - 0.5) * 2, f.a + (r() - 0.5) * 0.25, f.len * 0.72, f.w * 0.26, f.curl, 0.5);
  }
  ctx.fillStyle = t.light;
  ctx.globalAlpha = 0.75;
  for (const f of tufts) {
    if (f.hi && r() < 0.5) clump(ctx, f.x - 0.5, f.y - 0.5, f.a + (r() - 0.5) * 0.2, f.len * 0.55, f.w * 0.22, f.curl * 0.8, 0.5);
  }
  ctx.globalAlpha = 1;
}

// --- the four styles --------------------------------------------------------------------------
// Each returns {back, front, backMass, frontMass, hat: {y, s}} in head units (facing right).
const CX = 0, CY = -4, R = 20;
const around = (deg, rad) => [CX + Math.cos((deg * Math.PI) / 180) * rad, CY + Math.sin((deg * Math.PI) / 180) * rad];
const j = (r, amt) => (r() - 0.5) * 2 * amt;

/** The front cap: the part of the head above y = `edge`, slightly larger than the head. */
function capPath(edge, pad = 1.2) {
  return (ctx) => {
    const rr = R + pad;
    const dy = edge - CY;
    const half = Math.acos(Math.max(-1, Math.min(1, dy / rr)));
    ctx.beginPath();
    ctx.arc(CX, CY, rr, Math.PI / 2 + half, Math.PI / 2 - half + Math.PI * 2, false);
    ctx.closePath();
  };
}

function mop(r) {
  const back = [];
  // A round shaggy mop: soft clumps all over the crown, the sides hanging down over the ears.
  for (let deg = 140; deg <= 400; deg += 7.5) {
    const [x, y] = around(deg + j(r, 2), 22.5 + j(r, 1));
    const out = (deg * Math.PI) / 180;
    const side = Math.abs(Math.cos(out));
    const hang = side > 0.45;
    const a = hang ? Math.PI / 2 + Math.cos(out) * -0.3 + j(r, 0.18) : out + j(r, 0.22);
    back.push({ x, y, a, len: (hang ? 9.5 + (side - 0.45) * 4 : 9.5) + r() * 2.2, w: 9.5 + r() * 2, curl: j(r, 0.4), hi: y < CY - 14 && x < 4 });
  }
  const front = [];
  // A messy fringe falling forward, tips uneven, all above the eyes.
  for (let x = -18; x <= 18; x += 3) {
    const bx = x + j(r, 0.7);
    const top = CY - Math.sqrt(Math.max(0, R * R - bx * bx)) * 0.9;
    const by = top + 1;
    const tipY = -11.3 + j(r, 1.4) + Math.abs(bx) * 0.12;
    front.push({ x: bx, y: by, a: Math.PI / 2 + 0.16 + j(r, 0.2), len: Math.max(5, tipY - by), w: 7 + r() * 1.6, curl: j(r, 0.3), hi: bx < 0 });
  }
  // Crown fuzz, so the top is soft rather than a hard line.
  for (let deg = 208; deg <= 332; deg += 12) {
    const [x, y] = around(deg + j(r, 3), 19.5 + j(r, 1));
    front.push({ x, y, a: (deg * Math.PI) / 180 + j(r, 0.3), len: 5.5 + r() * 2, w: 7 + r() * 1.4, curl: j(r, 0.45), hi: x < 0 });
  }
  return { back, front, backMass: null, frontMass: capPath(-13.5), hat: { y: -34, s: 1.12 } };
}

function pom(r) {
  // A cloud of round puffs on the crown; the bigger puffs sit toward the back.
  const puffs = [[0, -30, 9.5], [-10.5, -27, 7.6], [10, -27.5, 7], [-5.8, -38.5, 7.6], [5.6, -38.8, 7], [-0.6, -45.5, 5.4]];
  const inOther = (x, y, i) => puffs.some(([cx, cy, rr], k) => k !== i && Math.hypot(x - cx, y - cy) < rr - 0.6);
  const front = [];
  puffs.forEach(([cx, cy, rr], i) => {
    const n = Math.round(rr * 2.7);
    for (let k = 0; k < n; k++) {
      const ang = (k / n) * Math.PI * 2 + j(r, 0.1);
      const x = cx + Math.cos(ang) * (rr - 1.2), y = cy + Math.sin(ang) * (rr - 1.2);
      if (inOther(x, y, i) || y > CY - 12.5) continue;
      front.push({ x, y, a: ang + j(r, 0.3), len: 3 + r() * 1.4, w: 4.6 + r() * 1.2, curl: j(r, 0.6), hi: false });
    }
  });
  const mass = (ctx) => {
    ctx.beginPath();
    for (const [cx, cy, rr] of puffs) { ctx.moveTo(cx + rr, cy); ctx.arc(cx, cy, rr, 0, Math.PI * 2); }
  };
  // Volume: a soft crease under each upper puff and a highlight on each, so it reads as a cloud.
  const extra = (ctx, tn, small) => {
    ctx.save();
    ctx.lineCap = "round";
    ctx.strokeStyle = tn.dark;
    ctx.lineWidth = small ? 1.6 : 1.3;
    for (const [cx, cy, rr] of puffs) {
      if (cy > -33) continue;
      ctx.beginPath(); ctx.arc(cx, cy, rr * 0.86, 0.35, Math.PI - 0.35); ctx.stroke();
    }
    ctx.fillStyle = tn.light;
    ctx.globalAlpha = small ? 0.55 : 0.7;
    for (const [cx, cy, rr] of puffs) {
      ctx.beginPath(); ctx.ellipse(cx - rr * 0.3, cy - rr * 0.38, rr * 0.38, rr * 0.22, -0.5, 0, Math.PI * 2); ctx.fill();
    }
    ctx.restore();
  };
  return { back: [], front, backMass: null, frontMass: mass, frontExtra: extra, hat: { y: -43, s: 0.95 } };
}

function fringe(r) {
  const back = [];
  // A fuzzy bowl hugging the head, down to ear level, shaggy at the hem.
  for (let deg = 172; deg <= 368; deg += 9) {
    const [x, y] = around(deg + j(r, 2), 20.5 + j(r, 0.6));
    const out = (deg * Math.PI) / 180;
    const hem = Math.abs(Math.cos(out)) > 0.8;
    back.push({ x, y, a: hem ? out + Math.PI / 2 * Math.sign(Math.cos(out)) * 0.5 + j(r, 0.2) : out + j(r, 0.2), len: (hem ? 5 : 4) + r() * 1.6, w: 7 + r() * 1.4, curl: j(r, 0.3), hi: y < CY - 12 && x < 2 });
  }
  const front = [];
  // Heavy bangs swept forward (+x), shaggy, ending just above the eyes.
  for (let x = -19.5; x <= 19.5; x += 2.6) {
    const bx = x + j(r, 0.5);
    const top = CY - Math.sqrt(Math.max(0, R * R - bx * bx)) * 0.96;
    const by = top + 1.5;
    const tipY = -9.9 + (Math.abs(bx) > 15 ? 3 + (Math.abs(bx) - 15) * 0.9 : 0) + j(r, 0.9);
    front.push({ x: bx, y: by, a: Math.PI / 2 - 0.24 + j(r, 0.12), len: Math.max(5, tipY - by), w: 6.6 + r() * 1.4, curl: j(r, 0.2) - 0.1, hi: bx < -2 && r() < 0.6 });
  }
  return { back, front, backMass: capPath(-6, 1.6), frontMass: capPath(-15.5, 0.6), hat: { y: -26, s: 1.05 } };
}

function crest(r) {
  const back = [];
  // A fuzzy ridge down the middle of the crown (the sides of the head stay bare), tallest just
  // behind the top, swept a little back (-x) with tips curling: a soft mohawk, not spikes.
  for (let deg = 236; deg <= 306; deg += 6) {
    const [x, y] = around(deg + j(r, 1.5), 18.8);
    const mid = Math.max(0, 1 - Math.abs(deg - 266) / 44);
    const a = (deg * Math.PI) / 180 - 0.16 + j(r, 0.1);
    const len = 8 + mid * 15 + r() * 2;
    for (const fan of [-0.2, 0.2, 0]) {
      back.push({ x: x + fan * 4, y, a: a + fan + j(r, 0.08), len: len * (fan ? 0.8 : 1), w: 7.4 + r() * 1.2, curl: -0.3 + j(r, 0.22), hi: fan < 0 && r() < 0.6 });
    }
  }
  // A little fuzz at the root, in front, so the crest grows out of hair, not the scalp.
  const front = [];
  for (let deg = 240; deg <= 302; deg += 7) {
    const [x, y] = around(deg + j(r, 2), 19.4);
    front.push({ x, y, a: (deg * Math.PI) / 180 - 0.2 + j(r, 0.3), len: 4.4 + r() * 1.6, w: 6.4 + r() * 1.2, curl: j(r, 0.5), hi: x < 0 });
  }
  return { back, front, backMass: null, frontMass: null, hat: { y: -23.5, s: 1 } };
}

const STYLES = { mop, pom, fringe, crest };

// --- head and accessories (same shapes as nyborg-draw.js) ------------------------------------
function head(ctx) {
  ctx.fillStyle = "rgba(0,0,0,0.18)";
  ctx.beginPath(); ctx.ellipse(0, 8, 20, 4.5, 0, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = NYBORG.head; ctx.strokeStyle = NYBORG.outline; ctx.lineWidth = 2.6;
  ctx.beginPath(); ctx.arc(CX, CY, R, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
}
function face(ctx) {
  for (const x of [-7, 7]) {
    ctx.fillStyle = NYBORG.eyes; ctx.beginPath(); ctx.arc(x, -3, 2.6, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = "#fff"; ctx.beginPath(); ctx.arc(x + 0.8, -3.9, 0.8, 0, Math.PI * 2); ctx.fill();
  }
}
function glasses(ctx) {
  ctx.save();
  ctx.strokeStyle = "#3B3632"; ctx.lineWidth = 1.5; ctx.fillStyle = "rgba(255,255,255,0.25)";
  for (const x of [-7, 7]) { ctx.beginPath(); ctx.arc(x, -3, 5.1, 0, Math.PI * 2); ctx.fill(); ctx.stroke(); }
  ctx.beginPath();
  ctx.moveTo(-1.9, -3.6); ctx.quadraticCurveTo(0, -5, 1.9, -3.6);
  ctx.moveTo(-12.1, -4.2); ctx.lineTo(-18.5, -6.2);
  ctx.moveTo(12.1, -4.2); ctx.lineTo(18.5, -6.2);
  ctx.stroke();
  ctx.restore();
}
function hat(ctx, { y, s }) {
  ctx.save();
  ctx.translate(0, y); ctx.rotate((8 * Math.PI) / 180); ctx.scale(s, s);
  ctx.strokeStyle = NYBORG.outline; ctx.lineWidth = 1.5; ctx.lineJoin = "round";
  ctx.fillStyle = "#5F7A99"; ctx.beginPath(); ctx.ellipse(0, 0, 15.5, 3.3, 0, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
  ctx.fillStyle = "#6E8BAE"; ctx.beginPath();
  ctx.moveTo(-8.4, -0.6); ctx.bezierCurveTo(-8.4, -8.4, -4.5, -11, 0, -11); ctx.bezierCurveTo(4.5, -11, 8.4, -8.4, 8.4, -0.6);
  ctx.closePath(); ctx.fill(); ctx.stroke();
  ctx.fillStyle = "#F5F2EC"; ctx.fillRect(-8, -4.2, 16, 2.8);
  ctx.restore();
}

/**
 * Draw a Nyborg with one of the prototype hair styles ("mop" | "pom" | "fringe" | "crest"), or
 * "yarn" for today's strands (delegates to nyborg-draw.js). Same arguments as drawNyborg, plus
 * `style` and `id` (seeds the jitter).
 */
export function drawNyborgHair(ctx, { hair = "#D9534F", style = "mop", id = "nyborg", facing = 1, bob = 0, scale = 0.55, strands = 3, accessories = [] } = {}) {
  if (style === "yarn") return drawNyborg(ctx, { hair, facing, bob, scale, strands, accessories });
  const make = STYLES[style] || STYLES.mop;
  const r = rng(seedFrom(`${id}:${style}`));
  const shape = make(r);
  const t = tones(hair);
  ctx.save();
  ctx.scale(facing * scale, scale);
  ctx.translate(0, bob / scale);
  // Device pixels per head unit decides the level of detail (viewer heads are ~20-30 px wide).
  const m = ctx.getTransform();
  const px = Math.hypot(m.a, m.b) * R * 2;
  const small = px < 34;
  ctx.lineJoin = "round";
  furLayer(ctx, shape.back, shape.backMass, t, small, r, shape.backExtra);
  head(ctx);
  furLayer(ctx, shape.front, shape.frontMass, t, small, r, shape.frontExtra);
  face(ctx);
  if (accessories.includes("glasses")) glasses(ctx);
  if (accessories.includes("hat")) hat(ctx, shape.hat);
  ctx.restore();
}
