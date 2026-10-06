// Shareable match links that carry Nyborgs: builds for the engine, looks and names for
// the viewer only. Pure (no DOM, no wasm); the engine is passed in, so Node tests use it too.
//
// Racing (docs/design/viewer-multi-game.md §4):
//   ?game=racing&seed=42&cars=cutter-5-2-2,follower-2-2-5
//   cars:  1–4 builds, each a behavior plus its stat levels in catalog order (power-top_speed-grip),
//          written like a tank loadout. No `cars` = today's default grid.
// Cosmetics, for any game's slots (racing cars 1–4; tank 1 = blue, 2 = orange):
//   &looks=D9534F-3-g,3F6FD8-2,&name1=Pip&name2=Juno
//   looks: one per slot, comma-separated: palette hex (no #), strands 2–4, then accessory
//          letters (g = glasses, h = hat). An empty entry keeps that slot's default look.
//   nameN: display name of slot N (1–24 characters).
// Cosmetics never reach the engine: only `seed` and `cars` build the race.
import { HAIR_PALETTE, ACCESSORIES } from "./nyborg-draw.js";
import { friendlyErrors } from "./build-tabs.js";

export const MAX_CARS = 4;
export const MAX_SLOTS = 4;
const ACC_CODE = { glasses: "g", hat: "h" };
const CODE_ACC = Object.fromEntries(Object.entries(ACC_CODE).map(([k, v]) => [v, k]));
const PALETTE = new Set(HAIR_PALETTE.map((c) => c.hex.toUpperCase()));

/** "cutter-5-2-2" from a build, stat levels in catalog order. */
export function encodeBuild(catalog, build) {
  return [build.behavior.id, ...catalog.stats.map((s) => build.levels[s.key])].join("-");
}

/** A build from "cutter-5-2-2", or null if it isn't written like one. The engine still checks it. */
export function decodeBuild(catalog, token) {
  const parts = String(token ?? "").trim().split("-");
  if (parts.length !== catalog.stats.length + 1 || !/^[a-z][a-z0-9_]*$/.test(parts[0])) return null;
  const nums = parts.slice(1);
  if (!nums.every((v) => /^\d{1,2}$/.test(v))) return null;
  return {
    rules_version: catalog.rules_version,
    levels: Object.fromEntries(catalog.stats.map((s, i) => [s.key, Number(nums[i])])),
    behavior: { kind: "scripted", id: parts[0] },
  };
}

/** "D9534F-3-gh" from a look ({hair_color, strands, accessories}). */
export function encodeLook(look) {
  const acc = ACCESSORIES.map((a) => a.id).filter((id) => look.accessories?.includes(id)).map((id) => ACC_CODE[id]).join("");
  const base = `${String(look.hair_color).replace(/^#/, "").toUpperCase()}-${look.strands}`;
  return acc ? `${base}-${acc}` : base;
}

/** A look from "D9534F-3-gh", or null. Hair must be a palette colour, as in save format 1. */
export function decodeLook(token) {
  const m = /^([0-9a-fA-F]{6})-([234])(?:-([gh]{0,2}))?$/.exec(String(token ?? "").trim());
  if (!m) return null;
  const hex = `#${m[1].toUpperCase()}`;
  if (!PALETTE.has(hex)) return null;
  const letters = [...(m[3] || "")];
  if (new Set(letters).size !== letters.length) return null;
  return {
    hair_color: hex,
    strands: Number(m[2]),
    accessories: ACCESSORIES.map((a) => a.id).filter((id) => letters.includes(ACC_CODE[id])),
  };
}

/** A display name for a link: trimmed, no control characters, 1–24 characters, else null. */
export function cleanLinkName(raw) {
  if (typeof raw !== "string") return null;
  const n = raw.replace(/[\u0000-\u001f\u007f]/g, "").trim();
  return n.length >= 1 && [...n].length <= 24 ? n : null;
}

/** The cosmetic part of a query: `looks=…&name1=…`, or "" when there's nothing to carry. */
export function cosmeticsQuery(slots) {
  const parts = [];
  if (slots.some((s) => s?.look)) parts.push(`looks=${slots.map((s) => (s?.look ? encodeLook(s.look) : "")).join(",")}`);
  slots.forEach((s, i) => {
    const n = cleanLinkName(s?.name);
    if (n) parts.push(`name${i + 1}=${encodeURIComponent(n)}`);
  });
  return parts.join("&");
}

/** Looks and names for `count` slots from a query, plus notes for anything unreadable. */
export function parseCosmetics(search, count) {
  const q = toParams(search);
  const looks = Array(count).fill(null);
  const names = Array(count).fill(null);
  const notes = [];
  if (q.has("looks")) {
    const tokens = q.get("looks").split(",");
    for (let i = 0; i < count && i < tokens.length; i++) {
      if (!tokens[i].trim()) continue;
      looks[i] = decodeLook(tokens[i]);
      if (!looks[i]) notes.push(`Slot ${i + 1}'s look isn't one the Customizer makes, so it keeps the default colours.`);
    }
  }
  for (let i = 0; i < count; i++) {
    if (q.has(`name${i + 1}`)) names[i] = cleanLinkName(q.get(`name${i + 1}`));
  }
  return { looks, names, notes };
}

/**
 * The racing query for a spec: {seed, builds, custom, looks?, names?}. `cars` is printed only
 * when the builds came from a link (custom), so the plain race keeps `?game=racing&seed=42`.
 */
export function raceQuery(catalog, spec, extras = {}) {
  const parts = ["game=racing", `seed=${encodeURIComponent(spec.seed)}`];
  if (spec.custom) parts.push(`cars=${spec.builds.map((b) => encodeBuild(catalog, b)).join(",")}`);
  const cos = cosmeticsQuery(spec.builds.map((_, i) => ({ look: spec.looks?.[i], name: spec.names?.[i] })));
  if (cos) parts.push(cos);
  if (extras.speed && extras.speed !== 1) parts.push(`speed=${extras.speed}`);
  if (extras.paused) parts.push("paused=1");
  if (extras.t) parts.push(`t=${extras.t}`);
  return parts.join("&");
}

/**
 * Read a racing link. engine = {catalog, defaultBuild(), validateBuild(build) -> {ok, errors}};
 * defaults = the builds for a link without `cars`. Every car from the link goes through
 * validateBuild; one that fails races defaultBuild() instead, with a note saying so.
 */
export function parseRaceQuery(search, engine, defaults) {
  const q = toParams(search);
  const seed = (q.get("seed") || "42").trim() || "42";
  const notes = [];
  let builds = defaults.map((b) => structuredClone(b));
  let custom = false;
  const raw = q.has("cars") ? q.get("cars").split(",").map((t) => t.trim()) : [];
  if (q.has("cars") && raw.every((t) => !t)) notes.push("The link has no cars, so this is the default race.");
  else if (raw.length) {
    if (raw.length > MAX_CARS) notes.push(`A race takes 1 to ${MAX_CARS} cars; the link had ${raw.length}, so the first ${MAX_CARS} race.`);
    custom = true;
    builds = raw.slice(0, MAX_CARS).map((token, i) => {
      const b = decodeBuild(engine.catalog, token);
      const r = b ? engine.validateBuild(b) : null;
      if (b && r.ok) return b;
      const why = b ? friendlyErrors(engine.catalog, r, b.levels).join(" ") : "It isn't written like behavior-power-top_speed-grip.";
      notes.push(`Car ${i + 1} (${token || "empty"}) isn't a valid build: ${why} It races the default build instead.`);
      return engine.defaultBuild();
    });
  }
  const cos = parseCosmetics(q, builds.length);
  return { game: "racing", seed, builds, custom, looks: cos.looks, names: cos.names, notes: [...notes, ...cos.notes] };
}

function toParams(search) {
  if (search instanceof URLSearchParams) return search;
  return new URLSearchParams(String(search ?? "").replace(/^\?/, ""));
}
