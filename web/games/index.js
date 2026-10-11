// The app's arenas. A game shows up in the app when the engine lists it (games()) AND it has a
// module here; nothing in the app's screens names a game. Order: how the home screen lists them.
import racing from "./racing.js";
import tank from "./tank.js";
import { buildFor } from "../nyborg-library.js";

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

/** You against the house: your Nyborg in slot 1, house slots after it. */
export function quickMatch(engine, mod, nyborg, seed = "42") {
  return { game: mod.id, seed: String(seed), slots: [slotFor(engine, nyborg, mod.id), ...mod.house(engine)].slice(0, mod.slots.max) };
}

/** The viewer link (arena.html) for a match, relative to /arena/play/. */
export function viewerLink(engine, mod, match, base = "../arena.html") {
  return `${base}?${mod.query(engine, match)}`;
}
