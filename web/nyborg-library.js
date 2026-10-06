// The Nyborg library: save format 1 and its localStorage layout, per
// docs/design/nyborg-library.md. Plain JS with no DOM, so it runs under Node too.
// Storage and the engine's validator are passed in, never imported, which keeps this
// testable and game-agnostic: games come from the engine's games(), not from this file.

import { HAIR_PALETTE, ACCESSORIES } from "./nyborg-draw.js";

export const FORMAT = 1; // the save format this code reads and writes
export const MAX_NYBORGS = 200;
export const MAX_IMPORT_BYTES = 1024 * 1024;
export const KEYS = {
  index: "arena.nyborg.index", // {format, order: [ids], selected}
  nyborg: (id) => `arena.nyborg.p.${id}`, // one Nyborg
  picks: "arena.nyborg.picks", // match slots
  backup: (n) => `arena.nyborg.backup.v${n}`, // one-time copy before a migration
};
export const DEFAULT_LOOK = Object.freeze({ hair_color: "#D9534F", strands: 3, accessories: [] });

const HAIR_HEXES = HAIR_PALETTE.map((c) => c.hex);
const ACCESSORY_IDS = ACCESSORIES.map((a) => a.id);
const ISO = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/;

const nowIso = () => new Date().toISOString().replace(/\.\d+Z$/, "Z");
const clone = (v) => JSON.parse(JSON.stringify(v));
export const newId = () => globalThis.crypto.randomUUID();

/** A trimmed name, 1-24 characters, or null. */
export function cleanName(name) {
  const n = typeof name === "string" ? name.trim() : "";
  return n.length >= 1 && [...n].length <= 24 ? n : null;
}

/** Shape check for one saved Nyborg, apart from its builds (the engine checks those). */
export function checkShape(n) {
  const errs = [];
  if (!n || typeof n !== "object" || Array.isArray(n)) return ["not an object"];
  if (!Number.isInteger(n.format) || n.format < 1) errs.push("format");
  if (typeof n.id !== "string" || !n.id) errs.push("id");
  if (cleanName(n.name) !== n.name) errs.push("name");
  if (!ISO.test(n.created ?? "")) errs.push("created");
  if (!ISO.test(n.updated ?? "")) errs.push("updated");
  const l = n.look;
  if (!l || typeof l !== "object") errs.push("look");
  else {
    if (!HAIR_HEXES.includes(l.hair_color)) errs.push("look.hair_color");
    if (![2, 3, 4].includes(l.strands)) errs.push("look.strands");
    if (!Array.isArray(l.accessories) || l.accessories.some((a) => !ACCESSORY_IDS.includes(a))
      || new Set(l.accessories).size !== l.accessories.length) errs.push("look.accessories");
  }
  if (!n.builds || typeof n.builds !== "object" || Array.isArray(n.builds)) errs.push("builds");
  return errs;
}

/**
 * The engine side, injected: `{games(): [{game, rules_version}], defaultBuild(game): build,
 * validateBuild(game, build): {ok, errors?}}`. The page wraps the wasm calls; tests pass fakes.
 */
export function buildFor(engine, nyborg, game) {
  return nyborg.builds[game] ? clone(nyborg.builds[game]) : engine.defaultBuild(game);
}

/** Per-game status: "ok", "rebuild" (rules_version_mismatch) or "invalid". */
export function buildStatus(engine, nyborg, game) {
  const r = engine.validateBuild(game, buildFor(engine, nyborg, game));
  if (r.ok) return { status: "ok", result: r };
  if (r.errors.some((e) => e.code === "rules_version_mismatch")) return { status: "rebuild", result: r };
  return { status: "invalid", result: r };
}

export function nextFreeName(library, base = "Nyborg") {
  const taken = new Set(library.map((n) => n.name));
  for (let i = library.length + 1; ; i++) if (!taken.has(`${base} ${i}`)) return `${base} ${i}`;
}

/** A new Nyborg: default look, and each game at its catalog default (stored, so it is explicit). */
export function createNyborg(engine, name) {
  const t = nowIso();
  const builds = {};
  for (const { game } of engine.games()) builds[game] = engine.defaultBuild(game);
  return { format: FORMAT, id: newId(), name, created: t, updated: t, look: clone(DEFAULT_LOOK), builds };
}

/** Duplicate: everything except the id; the copy is named "Pip copy" (base trimmed so it fits 24). */
export function duplicateNyborg(n) {
  const t = nowIso();
  const name = `${[...n.name].slice(0, 19).join("").trim()} copy`;
  return { ...clone(n), id: newId(), name, created: t, updated: t };
}

export const touch = (n) => ({ ...n, updated: nowIso() });

/**
 * Check a Nyborg from a file. Returns {nyborg, notes} or {error}. A bad build never loads:
 * it is replaced by the game's default build with a note (nyborg-library.md section 4), and a
 * rules_version_mismatch is kept as-is and marked "rebuild needed" (never fixed silently).
 */
export function sanitizeImported(engine, raw) {
  const shape = checkShape(raw);
  if (shape.length) return { error: `not a Nyborg (bad ${shape.join(", ")})` };
  if (raw.format > FORMAT) return { error: "made by a newer version of the Customizer" };
  const known = new Set(engine.games().map((g) => g.game));
  const n = { format: FORMAT, id: raw.id, name: raw.name, created: raw.created, updated: raw.updated, look: clone(raw.look), builds: {} };
  const notes = [];
  for (const [game, build] of Object.entries(raw.builds)) {
    if (!known.has(game)) { notes.push(`${game}: unknown game, left out`); continue; }
    const r = engine.validateBuild(game, build);
    if (r.ok) n.builds[game] = clone(build);
    else if (r.errors.some((e) => e.code === "rules_version_mismatch")) { n.builds[game] = clone(build); notes.push(`${game}: rebuild needed`); }
    else { n.builds[game] = engine.defaultBuild(game); notes.push(`${game}: invalid build replaced with the default`); }
  }
  return { nyborg: n, notes };
}

/** Parse an import file: one Nyborg, or a library export {format, exported, nyborgs}. */
export function parseImport(text) {
  if (typeof text !== "string" || text.length > MAX_IMPORT_BYTES) return { error: "file is larger than 1 MB" };
  let v;
  try { v = JSON.parse(text); } catch { return { error: "file isn't valid JSON" }; }
  if (v && Array.isArray(v.nyborgs)) {
    if (v.nyborgs.length > MAX_NYBORGS) return { error: `a file holds at most ${MAX_NYBORGS} Nyborgs` };
    return { items: v.nyborgs };
  }
  return { items: [v] };
}

export const exportOne = (n) => JSON.stringify(n, null, 2) + "\n";
export const exportAll = (list) => JSON.stringify({ format: FORMAT, exported: nowIso(), nyborgs: list }, null, 2) + "\n";
export const fileNameFor = (n) => `${n.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "nyborg"}.nyborg.json`;

/**
 * The library in storage (localStorage, or any {getItem, setItem, removeItem}). Each Nyborg
 * has its own key, so a failed write can't damage the others. `load()` never deletes:
 * unreadable entries are listed in `broken` for the page to offer "Download raw" / "Remove".
 */
export class Library {
  constructor(storage) {
    this.storage = storage;
    this.list = [];
    this.broken = []; // [{id, raw}]
    this.readOnly = new Set(); // ids saved by a newer format: shown, never overwritten
    this.selected = null;
  }

  load() {
    let index = null;
    try { index = JSON.parse(this.storage.getItem(KEYS.index) || "null"); } catch { index = null; }
    this.list = [];
    this.broken = [];
    for (const id of index?.order || []) {
      const raw = this.storage.getItem(KEYS.nyborg(id));
      let n = null;
      try { n = JSON.parse(raw); } catch { n = null; }
      if (!n || checkShape(n).length || n.id !== id) { this.broken.push({ id, raw }); continue; }
      if (n.format > FORMAT) this.readOnly.add(id);
      this.list.push(n);
    }
    this.selected = this.list.some((n) => n.id === index?.selected) ? index.selected : this.list[0]?.id ?? null;
    return !!index;
  }

  writeIndex() {
    const order = [...this.list.map((n) => n.id), ...this.broken.map((b) => b.id)];
    this.storage.setItem(KEYS.index, JSON.stringify({ format: FORMAT, order, selected: this.selected }));
  }

  get(id) { return this.list.find((n) => n.id === id) || null; }

  /** Insert or replace one Nyborg, then the index. Throws the storage error (e.g. quota). */
  save(n, { after } = {}) {
    if (this.readOnly.has(n.id)) throw new Error("made by a newer version; not overwritten");
    const i = this.list.findIndex((x) => x.id === n.id);
    if (i < 0 && this.list.length >= MAX_NYBORGS) throw new Error(`the library holds at most ${MAX_NYBORGS} Nyborgs`);
    this.storage.setItem(KEYS.nyborg(n.id), JSON.stringify(n));
    if (i >= 0) this.list[i] = n;
    else {
      const at = after ? this.list.findIndex((x) => x.id === after) : -1;
      if (at >= 0) this.list.splice(at + 1, 0, n); else this.list.push(n);
    }
    this.writeIndex();
    return n;
  }

  remove(id) {
    this.storage.removeItem(KEYS.nyborg(id));
    this.list = this.list.filter((n) => n.id !== id);
    this.broken = this.broken.filter((b) => b.id !== id);
    if (this.selected === id) this.selected = this.list[0]?.id ?? null;
    this.writeIndex();
  }

  select(id) { this.selected = id; this.writeIndex(); }

  /** First visit: one starter Nyborg with defaults. */
  seedIfEmpty(engine) {
    if (this.list.length || this.broken.length) return null;
    const n = createNyborg(engine, nextFreeName(this.list));
    this.selected = n.id;
    return this.save(n);
  }
}

/** In-memory storage, for tests and for when the browser blocks localStorage. */
export class MemoryStorage {
  constructor() { this.m = new Map(); }
  getItem(k) { return this.m.has(k) ? this.m.get(k) : null; }
  setItem(k, v) { this.m.set(k, String(v)); }
  removeItem(k) { this.m.delete(k); }
}
