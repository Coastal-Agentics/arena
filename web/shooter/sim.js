// Shooter sim: deterministic, seeded, headless. No DOM, so the page, the Train worker and
// the node trainer (train.mjs) all run the exact same code. Fixed step: 1/60 s.
export const W = 960, H = 640, DT = 1 / 60;
export const BUDGET = 12, STAT_MAX = 6;
export const STATS = ["speed", "fire", "damage", "shield"];
export const LAYOUTS = ["open", "maze", "ring"];
export const HULLS = ["dart", "delta", "wing"];
export const COLORS = ["#5FE1FF", "#FF5FD2", "#B6FF5F", "#FFC85F", "#A98BFF"];
export const TRAILS = ["line", "dots", "none"];

export function rng(seed) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

/** Every ship spends exactly BUDGET points; cosmetics are never read here. */
export function validBuild(b) {
  if (!b || typeof b !== "object") return false;
  let sum = 0;
  for (const k of STATS) { const v = b[k]; if (!Number.isInteger(v) || v < 0 || v > STAT_MAX) return false; sum += v; }
  return sum === BUDGET;
}
export const defaultBuild = () => ({ speed: 3, fire: 3, damage: 3, shield: 3 });
export function derive(b) {
  if (!validBuild(b)) b = defaultBuild();
  return { speed: 150 + 22 * b.speed, cooldown: 0.34 - 0.035 * b.fire, damage: 1 + 0.35 * b.damage, hp: 3 + b.shield };
}

// AI behaviour genes, each in [0,1]; decode() maps them to real ranges.
export const GENES = ["lead", "dodge", "strafe", "keep", "prioNear", "prioThreat", "edge"];
export const GEN0 = Object.fromEntries(GENES.map((g) => [g, 0.5]));
export function decode(p) {
  return { lead: p.lead, dodge: 30 + 190 * p.dodge, strafe: 2 * p.strafe - 1, keep: 40 + 300 * p.keep,
    prioNear: p.prioNear, prioThreat: p.prioThreat, edge: 20 + 160 * p.edge };
}

const TYPES = {
  drifter: { r: 14, hp: 2, speed: 55, pts: 10, color: "#5FE1FF" },
  chaser: { r: 11, hp: 1, speed: 125, pts: 15, color: "#FF5F7A" },
  shooter: { r: 13, hp: 3, speed: 70, pts: 25, color: "#FFC85F" },
  splitter: { r: 18, hp: 4, speed: 60, pts: 20, color: "#B6FF5F" },
  mini: { r: 8, hp: 1, speed: 135, pts: 5, color: "#B6FF5F" },
};
export const ENEMY_TYPES = ["drifter", "chaser", "shooter", "splitter"];

function mazeRocks(r) {
  const rocks = [];
  for (let i = 0; i < 9; i++) {
    const x = 140 + (i % 3) * 340 + (r() - 0.5) * 90, y = 130 + Math.floor(i / 3) * 190 + (r() - 0.5) * 60;
    if (Math.hypot(x - W / 2, y - H / 2) < 110) continue;
    rocks.push({ x, y, r: 26 + r() * 22 });
  }
  return rocks;
}

/**
 * pilots: [{build, ai?: genes|null, team}] — ai null means the step input comes from a human.
 * opts: {seed, layout, maxTime}
 */
export function createWorld({ seed = 1, layout = "open", pilots = [{ build: defaultBuild(), ai: GEN0 }], maxTime = 90 } = {}) {
  const r = rng(seed);
  const w = { seed, layout, r, t: 0, maxTime, wave: 0, waveClock: 0, over: false, score: 0,
    ring: Math.hypot(W, H) / 2, rocks: layout === "maze" ? mazeRocks(r) : [],
    ships: [], enemies: [], bullets: [], events: [] };
  pilots.forEach((p, i) => {
    const s = derive(p.build);
    w.ships.push({ id: i, x: W / 2 + (i - (pilots.length - 1) / 2) * 60, y: H / 2, vx: 0, vy: 0, ax: 1, ay: 0,
      hp: s.hp, maxHp: s.hp, stat: s, cd: 0, inv: 1, alive: true, score: 0, kills: 0, ai: p.ai ? decode(p.ai) : null });
  });
  return w;
}

function spawnWave(w) {
  w.wave++;
  const n = 4 + w.wave * 3;
  for (let i = 0; i < n; i++) {
    const pool = ENEMY_TYPES.slice(0, Math.min(4, 1 + Math.floor((w.wave + i) / 2)));
    const type = pool[Math.floor(w.r() * pool.length)];
    const a = w.r() * Math.PI * 2, d = Math.min(w.ring - 30, 420);
    let x = W / 2 + Math.cos(a) * d, y = H / 2 + Math.sin(a) * d;
    x = Math.max(20, Math.min(W - 20, x)); y = Math.max(20, Math.min(H - 20, y));
    addEnemy(w, type, x, y);
  }
  w.events.push({ k: "wave", n: w.wave });
}
function addEnemy(w, type, x, y) {
  const T = TYPES[type], a = w.r() * Math.PI * 2;
  w.enemies.push({ type, x, y, vx: Math.cos(a) * T.speed, vy: Math.sin(a) * T.speed, r: T.r, hp: T.hp + Math.floor(w.wave / 4), cd: 1 + w.r() * 2, alive: true });
}

function nearestShip(w, x, y) {
  let best = null, bd = 1e9;
  for (const s of w.ships) if (s.alive) { const d = Math.hypot(s.x - x, s.y - y); if (d < bd) { bd = d; best = s; } }
  return best;
}

/** Bot brain: returns an input {mx,my,ax,ay,fire}. Pure function of the world state. */
export function think(w, s) {
  const g = s.ai; let mx = 0, my = 0, tgt = null, best = -1e9;
  for (const e of w.enemies) {
    const d = Math.hypot(e.x - s.x, e.y - s.y);
    const threat = e.type === "chaser" || e.type === "mini" ? 1 : e.type === "shooter" ? 0.7 : 0.3;
    const sc = -d * (0.2 + g.prioNear) / 100 + threat * g.prioThreat * 3;
    if (sc > best) { best = sc; tgt = e; }
  }
  let ax = s.ax, ay = s.ay;
  if (tgt) {
    const d = Math.hypot(tgt.x - s.x, tgt.y - s.y), tt = (d / 520) * g.lead * 1.6;
    ax = tgt.x + tgt.vx * tt - s.x; ay = tgt.y + tgt.vy * tt - s.y;
    const ux = (tgt.x - s.x) / (d || 1), uy = (tgt.y - s.y) / (d || 1);
    const k = d < g.keep ? -1 : 0.6;
    mx += ux * k - uy * g.strafe; my += uy * k + ux * g.strafe;
  }
  const push = (x, y, rad, wgt) => {
    const dx = s.x - x, dy = s.y - y, d = Math.hypot(dx, dy);
    if (d < rad && d > 0) { const f = wgt * (1 - d / rad) / d; mx += dx * f * 3; my += dy * f * 3; }
  };
  for (const e of w.enemies) push(e.x, e.y, g.dodge + e.r, 1.4);
  for (const b of w.bullets) if (b.from < 0) push(b.x, b.y, g.dodge * 0.7, 1.8);
  for (const k of w.rocks) push(k.x, k.y, k.r + g.edge * 0.5, 2);
  const m = g.edge;
  if (s.x < m) mx += 2 * (1 - s.x / m); if (s.x > W - m) mx -= 2 * (1 - (W - s.x) / m);
  if (s.y < m) my += 2 * (1 - s.y / m); if (s.y > H - m) my -= 2 * (1 - (H - s.y) / m);
  if (w.layout === "ring") { const d = Math.hypot(s.x - W / 2, s.y - H / 2); if (d > w.ring - m) { mx -= (s.x - W / 2) / d * 3; my -= (s.y - H / 2) / d * 3; } }
  return { mx, my, ax, ay, fire: !!tgt };
}

/** Advance one fixed step. inputs[i] is used for ships with no AI. */
export function step(w, inputs = []) {
  if (w.over) return w;
  w.events.length = 0;
  w.t += DT;
  if (w.layout === "ring") w.ring = Math.max(170, w.ring - 6 * DT);
  // Next wave when the field is clear, or after 10 s regardless (pressure keeps runs short).
  w.waveClock += DT;
  if ((w.enemies.length === 0 && w.waveClock > (w.wave ? 1.2 : 0.4)) || w.waveClock > 10) { w.waveClock = 0; spawnWave(w); }

  for (const s of w.ships) {
    if (!s.alive) continue;
    const inp = s.ai ? think(w, s) : (inputs[s.id] || { mx: 0, my: 0, ax: s.ax, ay: s.ay, fire: false });
    let mx = inp.mx, my = inp.my; const ml = Math.hypot(mx, my);
    if (ml > 1) { mx /= ml; my /= ml; }
    s.vx += (mx * s.stat.speed - s.vx) * 0.18; s.vy += (my * s.stat.speed - s.vy) * 0.18;
    s.x = Math.max(12, Math.min(W - 12, s.x + s.vx * DT)); s.y = Math.max(12, Math.min(H - 12, s.y + s.vy * DT));
    for (const k of w.rocks) collideRock(s, k, 12);
    const al = Math.hypot(inp.ax, inp.ay); if (al > 0.01) { s.ax = inp.ax / al; s.ay = inp.ay / al; }
    s.cd -= DT; s.inv -= DT;
    if (inp.fire && s.cd <= 0) {
      s.cd = s.stat.cooldown;
      w.bullets.push({ x: s.x + s.ax * 14, y: s.y + s.ay * 14, vx: s.ax * 520, vy: s.ay * 520, life: 1.1, from: s.id, dmg: s.stat.damage });
      w.events.push({ k: "shot", id: s.id });
    }
    if (w.layout === "ring" && Math.hypot(s.x - W / 2, s.y - H / 2) > w.ring) hurt(w, s, 1);
  }

  for (const e of w.enemies) {
    const T = TYPES[e.type], s = nearestShip(w, e.x, e.y);
    if (s && e.type !== "drifter") {
      const dx = s.x - e.x, dy = s.y - e.y, d = Math.hypot(dx, dy) || 1;
      const want = e.type === "shooter" ? (d < 220 ? -1 : 1) : 1;
      e.vx += (dx / d * T.speed * want - e.vx) * 0.04; e.vy += (dy / d * T.speed * want - e.vy) * 0.04;
      if (e.type === "shooter") { e.cd -= DT; if (e.cd <= 0) { e.cd = 1.6 + w.r(); w.bullets.push({ x: e.x, y: e.y, vx: dx / d * 230, vy: dy / d * 230, life: 3, from: -1, dmg: 1 }); } }
    }
    e.x += e.vx * DT; e.y += e.vy * DT;
    if (e.x < e.r || e.x > W - e.r) { e.vx *= -1; e.x = Math.max(e.r, Math.min(W - e.r, e.x)); }
    if (e.y < e.r || e.y > H - e.r) { e.vy *= -1; e.y = Math.max(e.r, Math.min(H - e.r, e.y)); }
    for (const k of w.rocks) collideRock(e, k, e.r);
    for (const s2 of w.ships) if (s2.alive && Math.hypot(s2.x - e.x, s2.y - e.y) < e.r + 10) { hurt(w, s2, 1); e.hp -= 2; if (e.hp <= 0) kill(w, e, null); }
  }

  for (const b of w.bullets) {
    b.x += b.vx * DT; b.y += b.vy * DT; b.life -= DT;
    if (b.x < 0 || b.x > W || b.y < 0 || b.y > H) b.life = 0;
    for (const k of w.rocks) if (Math.hypot(b.x - k.x, b.y - k.y) < k.r) { b.life = 0; w.events.push({ k: "spark", x: b.x, y: b.y, c: "#8FA3C0" }); }
    if (b.life <= 0) continue;
    if (b.from >= 0) {
      for (const e of w.enemies) if (e.alive && Math.hypot(b.x - e.x, b.y - e.y) < e.r + 3) {
        b.life = 0; e.hp -= b.dmg; w.events.push({ k: "spark", x: b.x, y: b.y, c: TYPES[e.type].color });
        if (e.hp <= 0) kill(w, e, w.ships[b.from]); break;
      }
    } else for (const s of w.ships) if (s.alive && Math.hypot(b.x - s.x, b.y - s.y) < 11) { b.life = 0; hurt(w, s, 1); break; }
  }
  w.bullets = w.bullets.filter((b) => b.life > 0);
  w.enemies = w.enemies.filter((e) => e.alive);
  if (!w.ships.some((s) => s.alive) || w.t >= w.maxTime) { w.over = true; w.events.push({ k: "over" }); }
  return w;
}

function collideRock(o, k, r) {
  const dx = o.x - k.x, dy = o.y - k.y, d = Math.hypot(dx, dy) || 1;
  if (d < k.r + r) { o.x = k.x + dx / d * (k.r + r); o.y = k.y + dy / d * (k.r + r); o.vx *= -0.3; o.vy *= -0.3; }
}
function hurt(w, s, n) {
  if (s.inv > 0) return;
  s.hp -= n; s.inv = 0.9; w.events.push({ k: "hit", id: s.id, x: s.x, y: s.y });
  if (s.hp <= 0) { s.alive = false; w.events.push({ k: "boom", x: s.x, y: s.y, c: "#FFFFFF", big: true }); }
}
function kill(w, e, by) {
  if (!e.alive) return;
  e.alive = false; const T = TYPES[e.type];
  w.events.push({ k: "boom", x: e.x, y: e.y, c: T.color });
  if (by) { by.score += T.pts; by.kills++; }
  w.score += T.pts;
  if (e.type === "splitter") for (let i = 0; i < 2; i++) addEnemy(w, "mini", e.x + (i ? 10 : -10), e.y);
}
export const enemyColor = (t) => TYPES[t].color;

/** Fitness of a gene set: mean over seeds × layouts of score + 5·seconds survived. */
export function evaluate(genes, { seeds = [1, 2, 3], layouts = LAYOUTS, build = defaultBuild(), maxTime = 60 } = {}) {
  let total = 0, n = 0, waves = 0, time = 0, score = 0;
  for (const seed of seeds) for (const layout of layouts) {
    const w = createWorld({ seed, layout, pilots: [{ build, ai: genes }], maxTime });
    while (!w.over) step(w);
    total += w.score + 5 * w.t; score += w.score; waves += w.wave; time += w.t; n++;
  }
  return { fitness: total / n, score: score / n, waves: waves / n, time: time / n };
}

/** Genetic search. onGen({gen, best, mean}) is called after each generation. */
export function train({ seed = 7, pop = 20, gens = 12, evalSeeds = [1, 2, 3], maxTime = 45, onGen = () => {} } = {}) {
  const r = rng(seed);
  let people = [{ ...GEN0 }];
  while (people.length < pop) people.push(Object.fromEntries(GENES.map((g) => [g, r()])));
  let best = null;
  for (let gen = 0; gen < gens; gen++) {
    const scored = people.map((g) => ({ g, f: evaluate(g, { seeds: evalSeeds, maxTime }).fitness })).sort((a, b) => b.f - a.f);
    if (!best || scored[0].f > best.f) best = scored[0];
    onGen({ gen, best: best.f, mean: scored.reduce((a, s) => a + s.f, 0) / scored.length, genes: best.g });
    const elite = scored.slice(0, Math.max(2, pop >> 2)).map((s) => s.g);
    people = [...elite];
    while (people.length < pop) {
      const a = elite[Math.floor(r() * elite.length)], b = elite[Math.floor(r() * elite.length)];
      people.push(Object.fromEntries(GENES.map((k) => { let v = r() < 0.5 ? a[k] : b[k]; if (r() < 0.3) v += (r() - 0.5) * 0.3; return [k, Math.max(0, Math.min(1, v))]; })));
    }
  }
  return { genes: best.g, fitness: best.f };
}
