// Tank Arena viewer helpers with no DOM: URL <-> match spec, triangle geometry, readouts.
// The rules themselves (valid loadouts, snapping, stat tables) come from games/tank via
// the wasm (tankCatalogJson, snapLoadout, canonicalTankQuery), so they can't drift.

export const TEAM_KEYS = ["blue", "orange"];
export const PLACEHOLDER_BOTS = ["Chaser", "Wanderer"];

// Triangle corners in SVG units: Attack on top, Speed bottom-left, Defense bottom-right.
export const TRI = { A: [110, 14], S: [14, 186], D: [206, 186] };

/** "kiter-5-3-1" -> { behavior: "kiter", loadout: "5-3-1" } (already canonical). */
export function splitTank(s) {
  const i = s.indexOf("-");
  return i < 0 ? { behavior: s, loadout: "3-3-3" } : { behavior: s.slice(0, i), loadout: s.slice(i + 1) };
}

export function joinTank(t) {
  return `${t.behavior}-${t.loadout}`;
}

/**
 * Read the page URL's query. Tank Arena links use seed/blue/orange (canonicalised by the
 * wasm, so bad input throws); legacy built-in-bot links use seed/a/b.
 */
export function specFromQuery(search, canonicalTankQuery) {
  const p = new URLSearchParams(search);
  if (!p.has("blue") && !p.has("orange") && (p.has("a") || p.has("b"))) {
    const pick = (v, d) => PLACEHOLDER_BOTS.find((b) => b.toLowerCase() === String(v || "").toLowerCase()) || d;
    return { mode: "bots", seed: (p.get("seed") || "42").trim(), bots: [pick(p.get("a"), "Chaser"), pick(p.get("b"), "Wanderer")] };
  }
  const tankKeys = new URLSearchParams();
  for (const k of ["seed", "blue", "orange"]) if (p.has(k)) tankKeys.set(k, p.get(k));
  const canon = new URLSearchParams(canonicalTankQuery(tankKeys.toString()));
  return {
    mode: "tank",
    seed: canon.get("seed"),
    tanks: [splitTank(canon.get("blue")), splitTank(canon.get("orange"))],
  };
}

/** The shareable query for a spec, plus optional viewer extras ({ tab, speed, ... }). */
export function queryFromSpec(spec, extras = {}) {
  const p = new URLSearchParams();
  p.set("seed", spec.seed);
  if (spec.mode === "bots") {
    p.set("a", spec.bots[0]);
    p.set("b", spec.bots[1]);
  } else {
    p.set("blue", joinTank(spec.tanks[0]));
    p.set("orange", joinTank(spec.tanks[1]));
  }
  for (const [k, v] of Object.entries(extras)) if (v !== undefined && v !== null && v !== "") p.set(k, String(v));
  return p.toString();
}

/** Barycentric weights [attack, speed, defense] of point (x, y); may be negative outside. */
export function pointToWeights(x, y, tri = TRI) {
  const [ax, ay] = tri.A, [sx, sy] = tri.S, [dx, dy] = tri.D;
  const det = (sy - dy) * (ax - dx) + (dx - sx) * (ay - dy);
  const wa = ((sy - dy) * (x - dx) + (dx - sx) * (y - dy)) / det;
  const ws = ((dy - ay) * (x - dx) + (ax - dx) * (y - dy)) / det;
  return [wa, ws, 1 - wa - ws];
}

export function weightsToPoint(w, tri = TRI) {
  const [a, s, d] = w;
  return [a * tri.A[0] + s * tri.S[0] + d * tri.D[0], a * tri.A[1] + s * tri.S[1] + d * tri.D[1]];
}

/** Exact triangle point of a loadout "A-S-D": weight = (level - 1) / 6. */
export function loadoutWeights(loadout) {
  return loadout.split("-").map((v) => (Number(v) - 1) / 6);
}

export function levels(loadout) {
  const [attack, speed, defense] = loadout.split("-").map(Number);
  return { attack, speed, defense };
}

/** Readout numbers for `mine` against `theirs` (both "A-S-D"), from the catalog tables. */
export function readout(catalog, mine, theirs) {
  const m = levels(mine), t = levels(theirs);
  const damage = catalog.damage[m.attack - 1];
  const hp = catalog.max_hp[m.defense - 1];
  const oppHp = catalog.max_hp[t.defense - 1];
  const turnDegPerSec = (catalog.turn_rate[m.speed - 1] * 360 * 60) / 65536;
  return {
    damage,
    speed: catalog.max_speed[m.speed - 1],
    turnDegPerSec: Math.round(turnDegPerSec),
    hp,
    oppHp,
    hitsToKill: Math.ceil(oppHp / damage),
  };
}

/** Preset name for a loadout, or "" if it isn't one. */
export function presetName(catalog, loadout) {
  const p = catalog.presets.find(([, l]) => l === loadout);
  return p ? p[0] : "";
}
