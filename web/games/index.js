// The app's arenas. A game shows up in the app when the engine lists it (games()) AND it has a
// module here; nothing in the app's screens names a game. Order: how the home screen lists them.
import racing from "./racing.js?v=20261024a2";
import tank from "./tank.js?v=20261024a2";
import { buildFor } from "../nyborg-library.js?v=20261024a2";
import { HOUSE, houseById } from "../app/house.js?v=20261024a2";

export const GAME_MODULES = [racing, tank];

/** The modules for the games this engine has, in app order. */
export function arenas(engine, modules = GAME_MODULES) {
  const have = new Set(engine.games().map((g) => g.game));
  return modules.filter((m) => have.has(m.id));
}

/** One Nyborg as a match slot: its build for this game (engine-checked later), look and name. */
export function slotFor(engine, nyborg, game) {
  return { build: buildFor(engine, nyborg, game), look: nyborg.look, name: nyborg.name, id: nyborg.id };
}

/**
 * The Nyborg ids for a match: `ids` as asked (unknown ids dropped, at most slots.max), else you
 * plus the game's house Nyborgs; topped up with unused house Nyborgs to slots.min.
 */
export function lineup(mod, known, me, ids = []) {
  let out = ids.filter((id) => known(id)).slice(0, mod.slots.max);
  if (!out.length) out = [me, ...mod.house].filter((id) => id && known(id)).slice(0, mod.slots.max);
  for (const n of HOUSE) if (out.length < mod.slots.min && !out.includes(n.id)) out.push(n.id);
  return out;
}

/** A match from Nyborg ids: you, house Nyborgs or a mix, in slot order. */
export function matchFor(engine, mod, nyborgs, seed = "42") {
  return { game: mod.id, seed: String(seed), slots: nyborgs.map((n) => slotFor(engine, n, mod.id)) };
}

/** You against the house: your Nyborg in slot 1, the game's house Nyborgs after it. */
export function quickMatch(engine, mod, nyborg, seed = "42") {
  return matchFor(engine, mod, [nyborg, ...mod.house.map(houseById)].slice(0, mod.slots.max), seed);
}

/** The viewer link (arena.html) for a match, relative to /arena/play/. */
export function viewerLink(engine, mod, match, base = "../arena.html") {
  return `${base}?${mod.query(engine, match)}`;
}

/** The arena a viewer link plays: ?game=<id>, else Tank (the viewer's default). */
export function gameForQuery(engine, params, modules = GAME_MODULES) {
  const id = params.get("game") || "tank";
  return arenas(engine, modules).find((m) => m.id === id) || null;
}
