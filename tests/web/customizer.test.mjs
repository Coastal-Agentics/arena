// Node tests for the Customizer's pure modules (no browser):
//   node --test tests/web/customizer.test.mjs
// Save format 1 round trip and storage layout (docs/design/nyborg-library.md), import
// checks, and the schema-driven build tab against the real catalogs and a made-up third game.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync, games, catalogJson, defaultBuild, validateBuild } from "../../web/pkg/engine_wasm.js";
import {
  FORMAT, KEYS, DEFAULT_LOOK, upgrade, Library, MemoryStorage, checkShape, cleanName, createNyborg,
  duplicateNyborg, nextFreeName, sanitizeImported, parseImport, exportOne, exportAll, fileNameFor, buildStatus,
} from "../../web/nyborg-library.js";
import { tabModel, pointsSpent, setLevel, applyPreset, setBehavior, summaryLine, friendlyErrors, readoutFor } from "../../web/build-tabs.js";

const root = new URL("../../", import.meta.url);
initSync({ module: readFileSync(fileURLToPath(new URL("web/pkg/engine_wasm_bg.wasm", root))) });

const wasmEngine = {
  games: () => JSON.parse(games()),
  defaultBuild: (g) => JSON.parse(defaultBuild(g)),
  validateBuild: (g, b) => JSON.parse(validateBuild(g, JSON.stringify(b))),
};

// A third game the engine doesn't have: four stats, budget 10, one stat costing 2 per level.
const KITE = {
  game: "kite", rules_version: 3, budget: 10,
  stats: [
    { key: "lift", label: "Lift", min: 1, max: 4, cost_per_level: 1, values: [1, 2, 3, 4].map((level) => ({ level, lift_n: level * 10 })) },
    { key: "drag", label: "Drag", min: 0, max: 3, cost_per_level: 1, values: [0, 1, 2, 3].map((level) => ({ level, drag_coeff: level / 10 })) },
    { key: "line_length", label: "Line length", min: 1, max: 5, cost_per_level: 2, values: [] },
    { key: "colour_pop", label: "Colour pop", min: 1, max: 2, cost_per_level: 1, values: [] },
  ],
  behaviors: ["swooper", "hoverer"],
  presets: [{ id: "calm", label: "Calm", levels: { lift: 3, drag: 2, line_length: 2, colour_pop: 1 } }],
  default_build: { rules_version: 3, levels: { lift: 3, drag: 2, line_length: 2, colour_pop: 1 }, behavior: { kind: "scripted", id: "hoverer" } },
};
// A fake engine with tank, racing and kite, validating kite with the catalog's own rules.
const fakeEngine = {
  games: () => [...wasmEngine.games(), { game: "kite", rules_version: 3 }],
  defaultBuild: (g) => (g === "kite" ? structuredClone(KITE.default_build) : wasmEngine.defaultBuild(g)),
  validateBuild: (g, b) => {
    if (g !== "kite") return wasmEngine.validateBuild(g, b);
    if (b.rules_version !== 3) return { ok: false, errors: [{ code: "rules_version_mismatch", key: "rules_version" }] };
    const errors = [];
    for (const s of KITE.stats) if (!Number.isInteger(b.levels?.[s.key]) || b.levels[s.key] < s.min || b.levels[s.key] > s.max) errors.push({ code: "out_of_range", key: s.key });
    if (!errors.length) {
      const p = pointsSpent(KITE, b.levels);
      if (p > KITE.budget) errors.push({ code: "over_budget", key: "levels" });
      if (p < KITE.budget) errors.push({ code: "under_budget", key: "levels" });
    }
    if (!KITE.behaviors.includes(b.behavior?.id)) errors.push({ code: "unknown_behavior", key: "behavior" });
    return errors.length ? { ok: false, errors } : { ok: true };
  },
};

test("save format 2: create, store, reload, byte-equal", () => {
  const s = new MemoryStorage();
  const lib = new Library(s);
  assert.equal(lib.load(), false, "empty storage has no index");
  const starter = lib.seedIfEmpty(wasmEngine);
  assert.equal(starter.name, "Nyborg 1");
  assert.equal(starter.format, FORMAT);
  assert.deepEqual(starter.look, DEFAULT_LOOK);
  assert.deepEqual(Object.keys(starter), ["format", "id", "name", "created", "updated", "look", "builds"]);
  assert.deepEqual(Object.keys(starter.builds), ["tank", "racing"]);
  for (const g of ["tank", "racing"]) assert.deepEqual(starter.builds[g], wasmEngine.defaultBuild(g));
  assert.equal(lib.seedIfEmpty(wasmEngine), null, "seeds only once");

  const second = createNyborg(wasmEngine, nextFreeName(lib.list));
  second.look = { hair_style: "pom", hair_color: "#3F6FD8", strands: 4, accessories: ["glasses", "hat"] };
  second.builds.tank = { rules_version: 1, levels: { attack: 4, speed: 3, defense: 2 }, behavior: { kind: "scripted", id: "kiter" } };
  lib.save(second);

  const index = JSON.parse(s.getItem("arena.nyborg.index"));
  assert.deepEqual(index, { format: 2, order: [starter.id, second.id], selected: starter.id });
  assert.equal(s.getItem(`arena.nyborg.p.${second.id}`), JSON.stringify(second));
  assert.equal(KEYS.picks, "arena.nyborg.picks");
  assert.equal(KEYS.backup(1), "arena.nyborg.backup.v1");

  const again = new Library(s);
  assert.equal(again.load(), true);
  assert.deepEqual(again.list, [starter, second]);
  assert.equal(buildStatus(wasmEngine, again.get(second.id), "tank").status, "ok");

  // Export and import round trip, one and all.
  assert.deepEqual(JSON.parse(exportOne(second)), second);
  const all = JSON.parse(exportAll(again.list));
  assert.equal(all.format, 2);
  assert.deepEqual(parseImport(exportAll(again.list)).items, [starter, second]);
  assert.deepEqual(sanitizeImported(wasmEngine, JSON.parse(exportOne(second))), { nyborg: second, notes: [] });
  assert.equal(fileNameFor({ name: "Pip the 2nd!" }), "pip-the-2nd.nyborg.json");
});

test("library: duplicate, names, delete, corrupt entries are kept", () => {
  const s = new MemoryStorage();
  const lib = new Library(s);
  const a = lib.seedIfEmpty(wasmEngine);
  const copy = duplicateNyborg({ ...a, name: "Pip" });
  assert.notEqual(copy.id, a.id);
  assert.equal(copy.name, "Pip copy");
  assert.equal(duplicateNyborg({ ...a, name: "x".repeat(24) }).name, "x".repeat(19) + " copy", "fits in 24");
  lib.save(copy, { after: a.id });
  assert.equal(nextFreeName(lib.list), "Nyborg 3");
  assert.equal(cleanName("  Pip  "), "Pip");
  assert.equal(cleanName(""), null);
  assert.equal(cleanName("x".repeat(25)), null);

  s.setItem(KEYS.nyborg(copy.id), "{not json");
  const re = new Library(s);
  re.load();
  assert.deepEqual(re.list.map((n) => n.id), [a.id]);
  assert.deepEqual(re.broken.map((b) => b.id), [copy.id]);
  assert.equal(s.getItem(KEYS.nyborg(copy.id)), "{not json", "never deleted automatically");
  re.remove(a.id);
  assert.equal(s.getItem(KEYS.nyborg(a.id)), null);
  assert.deepEqual(JSON.parse(s.getItem(KEYS.index)).order, [copy.id], "broken entry stays indexed");
});

test("import: shape checks, bad builds replaced, rules mismatch kept, newer format refused", () => {
  const good = createNyborg(wasmEngine, "Pip");
  assert.deepEqual(checkShape(good), []);
  assert.ok(checkShape({ ...good, look: { ...good.look, hair_color: "#00FF00" } }).includes("look.hair_color"), "green is not a yarn");
  assert.ok(checkShape({ ...good, look: { ...good.look, strands: 5 } }).includes("look.strands"));
  assert.ok(checkShape({ ...good, look: { ...good.look, accessories: ["hat", "hat"] } }).includes("look.accessories"));
  assert.ok(checkShape({ ...good, name: "<b>" + "x".repeat(30) }).includes("name"));

  const cheat = structuredClone(good);
  cheat.builds.tank.levels = { attack: 5, speed: 5, defense: 5 };
  cheat.builds.racing.rules_version = 99;
  cheat.builds.chess = { rules_version: 1 };
  const r = sanitizeImported(wasmEngine, cheat);
  assert.deepEqual(r.nyborg.builds.tank, wasmEngine.defaultBuild("tank"), "over budget never loads");
  assert.equal(r.nyborg.builds.racing.rules_version, 99, "kept for 'rebuild needed'");
  assert.equal(r.nyborg.builds.chess, undefined);
  assert.deepEqual(r.notes, ["tank: invalid build replaced with the default", "racing: rebuild needed", "chess: unknown game, left out"]);
  assert.equal(buildStatus(wasmEngine, r.nyborg, "racing").status, "rebuild");

  assert.match(sanitizeImported(wasmEngine, { ...good, format: 3 }).error, /newer version/);
  assert.match(sanitizeImported(wasmEngine, [1, 2]).error, /not a Nyborg/);
  assert.match(parseImport("{nope").error, /valid JSON/);
  assert.match(parseImport(JSON.stringify({ nyborgs: Array(201).fill(good) })).error, /at most 200/);
});

test("build tab: the real catalogs, budget 9, friendly errors", () => {
  const list = JSON.parse(games()).map((g) => g.game);
  assert.deepEqual(list, ["tank", "racing"]);
  for (const g of list) {
    const cat = JSON.parse(catalogJson(g));
    const b = JSON.parse(defaultBuild(g));
    const m = tabModel(cat, b);
    assert.equal(m.budget, 9);
    assert.equal(m.spent, 9);
    assert.equal(m.remaining, 0);
    assert.equal(m.stats.length, cat.stats.length);
    assert.deepEqual(m.behaviors.map((x) => x.id), cat.behaviors);
    const first = cat.stats[0].key;
    const over = setLevel(cat, b, first, 5);
    const res = wasmEngine.validateBuild(g, over);
    assert.equal(res.ok, false);
    assert.deepEqual(friendlyErrors(cat, res, over.levels), ["That's more than 9 points. Take a point off a stat."]);
    assert.equal(tabModel(cat, over).remaining, -2);
    const under = setLevel(cat, b, first, 1);
    assert.deepEqual(friendlyErrors(cat, wasmEngine.validateBuild(g, under), under.levels), ["2 points left to spend. Every Nyborg uses exactly 9."]);
    for (const p of cat.presets) assert.ok(wasmEngine.validateBuild(g, applyPreset(b, p)).ok, `${g} preset ${p.id}`);
    for (const id of cat.behaviors) assert.ok(wasmEngine.validateBuild(g, setBehavior(b, id)).ok);
  }
  const tank = JSON.parse(catalogJson("tank"));
  assert.equal(readoutFor(tank.stats[0], 4), "24 damage");
  assert.equal(summaryLine(tank, { levels: { attack: 4, speed: 3, defense: 2 }, behavior: { kind: "scripted", id: "kiter" } }), "Kiter 4/3/2");
});

test("build tab: a third game needs no Customizer change", () => {
  // Nothing game-specific is written in the tab code: no game id, stat key or behavior.
  const src = readFileSync(fileURLToPath(new URL("web/build-tabs.js", root)), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");
  for (const g of ["tank", "racing"]) {
    const cat = JSON.parse(catalogJson(g));
    for (const word of [g, ...cat.stats.map((s) => s.key), ...cat.behaviors]) {
      assert.ok(!new RegExp(`\\b${word}\\b`, "i").test(src), `build-tabs.js names "${word}"`);
    }
  }
  assert.ok(!/\b9\b/.test(src), "no hardcoded budget");

  // The made-up kite game: four stats, budget 10, a 2-point stat, a stat whose min is 0.
  const b = fakeEngine.defaultBuild("kite");
  const m = tabModel(KITE, b);
  assert.deepEqual([m.budget, m.spent, m.remaining], [10, 10, 0]);
  assert.deepEqual(m.stats.map((s) => [s.key, s.label, s.min, s.max, s.cost, s.level]),
    [["lift", "Lift", 1, 4, 1, 3], ["drag", "Drag", 0, 3, 1, 2], ["line_length", "Line length", 1, 5, 2, 2], ["colour_pop", "Colour pop", 1, 2, 1, 1]]);
  assert.equal(m.stats[0].readout, "30 lift n");
  assert.deepEqual(m.behaviors, [{ id: "swooper", label: "Swooper" }, { id: "hoverer", label: "Hoverer" }]);
  assert.equal(m.presets[0].on, true);
  assert.equal(setLevel(KITE, b, "lift", 9).levels.lift, 4, "clamped to max");
  assert.equal(setLevel(KITE, b, "drag", -3).levels.drag, 0, "clamped to min");
  const pricey = setLevel(KITE, b, "line_length", 3); // +2 points
  assert.equal(tabModel(KITE, pricey).remaining, -2);
  assert.deepEqual(friendlyErrors(KITE, fakeEngine.validateBuild("kite", pricey), pricey.levels), ["That's more than 10 points. Take a point off a stat."]);
  assert.deepEqual(friendlyErrors(KITE, { ok: false, errors: [{ code: "out_of_range", key: "line_length" }] }, b.levels), ["Line length must be between 1 and 5."]);
  assert.equal(summaryLine(KITE, b), "Hoverer 3/2/2/1");

  // The library seeds and imports a kite build like any other game.
  const lib = new Library(new MemoryStorage());
  const n = lib.seedIfEmpty(fakeEngine);
  assert.deepEqual(Object.keys(n.builds), ["tank", "racing", "kite"]);
  const cheat = structuredClone(n);
  cheat.builds.kite.levels.line_length = 5;
  assert.deepEqual(sanitizeImported(fakeEngine, cheat).nyborg.builds.kite, KITE.default_build);
});

test("format 1 -> 2: backup first, every Nyborg gets the Shaggy mop, strands kept", () => {
  const s = new MemoryStorage();
  const v1 = (id, name, look) => ({ format: 1, id, name, created: "2026-10-06T22:00:00Z", updated: "2026-10-06T22:00:00Z", look, builds: { tank: wasmEngine.defaultBuild("tank") } });
  const a = v1("a-1", "Pip", { hair_color: "#3F6FD8", strands: 4, accessories: ["hat"] });
  const b = v1("b-2", "Juno", { hair_color: "#F2C14E", strands: 2, accessories: [] });
  const index = { format: 1, order: ["a-1", "b-2"], selected: "b-2" };
  s.setItem(KEYS.index, JSON.stringify(index));
  s.setItem(KEYS.nyborg("a-1"), JSON.stringify(a));
  s.setItem(KEYS.nyborg("b-2"), JSON.stringify(b));
  const rawA = s.getItem(KEYS.nyborg("a-1"));
  // Record write order: the backup must land before any Nyborg is rewritten.
  const writes = [];
  const spy = { getItem: (k) => s.getItem(k), removeItem: (k) => s.removeItem(k), setItem: (k, v) => { writes.push(k); s.setItem(k, v); } };
  const lib = new Library(spy);
  lib.load();
  assert.equal(writes[0], "arena.nyborg.backup.v1");
  assert.deepEqual(writes.slice(1), [KEYS.nyborg("a-1"), KEYS.nyborg("b-2"), KEYS.index]);
  const backup = JSON.parse(s.getItem("arena.nyborg.backup.v1"));
  assert.deepEqual(backup.index, index);
  assert.equal(backup.nyborgs["a-1"], rawA, "the backup holds the exact format-1 text");
  const up = JSON.parse(s.getItem(KEYS.nyborg("a-1")));
  assert.deepEqual(up, { ...a, format: 2, look: { hair_style: "mop", hair_color: "#3F6FD8", strands: 4, accessories: ["hat"] } });
  assert.equal(lib.selected, "b-2");
  assert.deepEqual(lib.migration, { from: 1, to: 2, count: 2, backup: "arena.nyborg.backup.v1", error: null });
  assert.equal(JSON.parse(s.getItem(KEYS.index)).format, 2);
  // Loading again: nothing to migrate, and the backup is never overwritten.
  const again = new Library(spy);
  writes.length = 0;
  again.load();
  assert.deepEqual(writes, []);
  assert.equal(again.migration, undefined);
  assert.deepEqual(again.list.map((n) => n.look.hair_style), ["mop", "mop"]);
  // upgrade() leaves format 2 alone.
  assert.equal(upgrade(up), up);
});

test("format 1 -> 2: if the backup can't be written, nothing is written", () => {
  const s = new MemoryStorage();
  const n = { format: 1, id: "a-1", name: "Pip", created: "2026-10-06T22:00:00Z", updated: "2026-10-06T22:00:00Z", look: { hair_color: "#3F6FD8", strands: 3, accessories: [] }, builds: {} };
  s.setItem(KEYS.index, JSON.stringify({ format: 1, order: ["a-1"], selected: "a-1" }));
  s.setItem(KEYS.nyborg("a-1"), JSON.stringify(n));
  const before = new Map(s.m);
  const full = { getItem: (k) => s.getItem(k), removeItem: () => {}, setItem: () => { const e = new Error("full"); e.name = "QuotaExceededError"; throw e; } };
  const lib = new Library(full);
  lib.load();
  assert.equal(lib.migration.error.name, "QuotaExceededError");
  assert.equal(lib.get("a-1").look.hair_style, "mop", "upgraded in memory");
  assert.deepEqual(new Map(s.m), before, "storage untouched");
});

test("format 2 shape: hair_style required and checked; format-1 imports upgrade with a note", () => {
  const n = createNyborg(wasmEngine, "Pip");
  assert.deepEqual(checkShape(n), []);
  assert.ok(checkShape({ ...n, look: { ...n.look, hair_style: "broccoli" } }).includes("look.hair_style"));
  const { hair_style, ...v1look } = n.look;
  assert.ok(checkShape({ ...n, look: v1look }).includes("look.hair_style"));
  assert.deepEqual(checkShape({ ...n, format: 1, look: v1look }), [], "format 1 needs no hair_style");
  const r = sanitizeImported(wasmEngine, { ...n, format: 1, look: { ...v1look, strands: 2 } });
  assert.equal(r.nyborg.format, 2);
  assert.deepEqual(r.nyborg.look, { hair_style: "mop", hair_color: n.look.hair_color, strands: 2, accessories: [] });
  assert.match(r.notes[0], /Shaggy mop/);
});
