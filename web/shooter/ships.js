// Shooter ship library. Lives alongside the Nyborg library under its own key
// (arena.shooter.ships) and only ever *reads* Nyborg keys, so the Nyborg save format is untouched.
import { KEYS } from "../nyborg-library.js";
import { BUDGET, COLORS, HULLS, STATS, TRAILS, defaultBuild, validBuild } from "./sim.js";

export const SHIP_KEY = "arena.shooter.ships";
export const SHIP_FORMAT = 1;
export const defaultShip = () => ({ name: "Skiff", build: defaultBuild(), look: { hull: HULLS[0], color: COLORS[0], trail: TRAILS[0] }, pilot: null });

/** Coerce anything into a valid ship; unknown fields are dropped, bad ones reset. */
export function cleanShip(s) {
  const d = defaultShip();
  if (!s || typeof s !== "object") return d;
  const look = s.look || {};
  return {
    name: typeof s.name === "string" && s.name.trim() ? s.name.trim().slice(0, 24) : d.name,
    build: validBuild(s.build) ? Object.fromEntries(STATS.map((k) => [k, s.build[k]])) : d.build,
    look: { hull: HULLS.includes(look.hull) ? look.hull : d.look.hull, color: COLORS.includes(look.color) ? look.color : d.look.color, trail: TRAILS.includes(look.trail) ? look.trail : d.look.trail },
    pilot: typeof s.pilot === "string" ? s.pilot : null,
  };
}
export const spent = (b) => STATS.reduce((a, k) => a + (b[k] | 0), 0);
export const left = (b) => BUDGET - spent(b);

export function loadShip(storage) {
  try { const v = JSON.parse(storage.getItem(SHIP_KEY) || "null"); if (v && v.format === SHIP_FORMAT) return cleanShip(v.ship); } catch {}
  return defaultShip();
}
export function saveShip(storage, ship) {
  const s = cleanShip(ship);
  storage.setItem(SHIP_KEY, JSON.stringify({ format: SHIP_FORMAT, ship: s }));
  return s;
}
/** Nyborgs available as pilots: [{id, name}], read-only. */
export function pilots(storage) {
  try {
    const idx = JSON.parse(storage.getItem(KEYS.index) || "null");
    return (idx?.order || []).map((id) => { try { const n = JSON.parse(storage.getItem(KEYS.nyborg(id))); return n?.name ? { id, name: n.name } : null; } catch { return null; } }).filter(Boolean);
  } catch { return []; }
}
