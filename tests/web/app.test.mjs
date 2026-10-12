// Node tests for the Nyborg app (web/app/, web/games/): routes, the arena registry, and the
// links the home screen builds playing exactly the matches a direct engine run plays.
//   node --test "tests/web/**/*.test.mjs"
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import * as wasm from "../../web/pkg/engine_wasm.js";
import { makeEngine } from "../../web/app/engine.js";
import { parseHash, href, matchRoute } from "../../web/app/router.js";
import { GAME_MODULES, arenas, slotFor, quickMatch, viewerLink, lineup, matchFor, gameForQuery } from "../../web/games/index.js";
import { createNyborg, checkShape } from "../../web/nyborg-library.js";
import { HOUSE, houseById, isHouse } from "../../web/app/house.js";
import { parseRaceQuery } from "../../web/nyborg-link.js";
import { specFromQuery, queryFromSpec } from "../../web/tank-ui.js";
import { defaultRaceBuilds } from "../../web/race-ui.js";

const root = new URL("../../", import.meta.url);
wasm.initSync({ module: readFileSync(fileURLToPath(new URL("web/pkg/engine_wasm_bg.wasm", root))) });
const engine = makeEngine(wasm);
const raceEngine = { catalog: engine.catalog("racing"), defaultBuild: () => engine.defaultBuild("racing"), validateBuild: (b) => engine.validateBuild("racing", b) };

const pip = () => {
  const n = createNyborg(engine, "Pip");
  n.look = { hair_style: "mop", hair_color: "#3F6FD8", strands: 3, accessories: ["hat"] };
  n.builds.racing = { rules_version: 1, levels: { power: 5, top_speed: 2, grip: 2 }, behavior: { kind: "scripted", id: "cutter" } };
  n.builds.tank = { rules_version: 1, levels: { attack: 4, speed: 3, defense: 2 }, behavior: { kind: "scripted", id: "kiter" } };
  return n;
};
const runRace = (seed, builds) => { const r = wasm.WasmRace.fromBuilds(seed, JSON.stringify(builds)); r.step(1_000_000); return [r.stateHash(), r.outcomeJson()]; };
const runTank = (q) => { const m = wasm.WasmMatch.tank(q); m.step(1_000_000); return [m.stateHash(), m.stateJson()]; };

test("routes: parse, build and match hash routes", () => {
  assert.deepEqual(parseHash("").path, []);
  assert.deepEqual(parseHash("#/").path, []);
  const p = parseHash("#/play/racing?me=abc&seed=7");
  assert.deepEqual(p.path, ["play", "racing"]);
  assert.equal(p.params.get("me"), "abc");
  assert.equal(href(["nyborg", "a b"], "x=1"), "#/nyborg/a%20b?x=1");
  assert.deepEqual(parseHash(href(["nyborg", "a b"])).path, ["nyborg", "a b"]);
  const routes = [{ pattern: "" }, { pattern: "new" }, { pattern: "nyborg/:id" }, { pattern: "nyborg/:id/:game" }];
  assert.equal(matchRoute(routes, []).route.pattern, "");
  assert.deepEqual(matchRoute(routes, ["nyborg", "x", "tank"]).args, { id: "x", game: "tank" });
  assert.equal(matchRoute(routes, ["nope"]), null);
});

test("arenas: every engine game with a module, in app order; a third game plugs in", () => {
  assert.deepEqual(arenas(engine).map((m) => m.id), ["racing", "tank"]);
  for (const m of GAME_MODULES) for (const k of ["id", "title", "button", "start", "setup", "blurb", "slots", "house", "query"]) assert.ok(k in m, `${m.id}.${k}`);
  const third = { id: "golf", title: "Golf", button: "Golf ▶", slots: { min: 1, max: 1, names: ["You"] }, house: [], query: () => "game=golf" };
  const fakeEngine = { ...engine, games: () => [...engine.games(), { game: "golf", rules_version: 1 }] };
  assert.deepEqual(arenas(fakeEngine, [...GAME_MODULES, third]).map((m) => m.id), ["racing", "tank", "golf"]);
  assert.deepEqual(arenas(engine, [...GAME_MODULES, third]).map((m) => m.id), ["racing", "tank"], "no engine game, no arena");
});

test("house Nyborgs: named, valid format-2 Nyborgs, mixed hair, every build on the same budget", () => {
  assert.ok(HOUSE.length >= 4 && HOUSE.length <= 8, "a short cast");
  assert.equal(new Set(HOUSE.map((n) => n.name)).size, HOUSE.length);
  assert.ok(new Set(HOUSE.map((n) => n.look.hair_style)).size >= 4, "mixed across hair styles");
  for (const n of HOUSE) {
    assert.ok(isHouse(n.id) && houseById(n.id) === n);
    assert.deepEqual(checkShape(n), [], n.name);
    assert.ok(n.tagline.length > 0 && n.tagline.length < 70);
    for (const { game } of engine.games()) {
      assert.equal(engine.validateBuild(game, n.builds[game]).ok, true, `${n.name} ${game}`);
      const sum = Object.values(n.builds[game].levels).reduce((a, b) => a + b, 0);
      assert.equal(sum, engine.catalog(game).budget, `${n.name} ${game} spends the budget`);
    }
  }
  for (const m of GAME_MODULES) for (const id of m.house) assert.ok(houseById(id), `${m.id} house ${id}`);
});

test("lineup: you plus the house by default; asked ids kept; topped up to the minimum", () => {
  const racing = GAME_MODULES.find((m) => m.id === "racing");
  const tank = GAME_MODULES.find((m) => m.id === "tank");
  const mine = new Set(["me", "pal"]);
  const known = (id) => mine.has(id) || !!houseById(id);
  assert.deepEqual(lineup(racing, known, "me"), ["me", ...racing.house]);
  assert.deepEqual(lineup(tank, known, "me"), ["me", "house-pepper"]);
  assert.deepEqual(lineup(racing, known, "me", ["pal", "nope", "house-mumble"]), ["pal", "house-mumble"]);
  assert.deepEqual(lineup(tank, known, "me", ["me", "pal", "house-biscuit"]), ["me", "pal"], "at most slots.max");
  assert.deepEqual(lineup(tank, known, "me", ["pal"]), ["pal", HOUSE[0].id], "topped up to slots.min");
});

test("racing quick match: you plus Biscuit, Noodle and Captain Fuzz; the link plays the direct race", () => {
  const n = pip();
  const racing = GAME_MODULES.find((m) => m.id === "racing");
  const m = quickMatch(engine, racing, n);
  assert.equal(m.slots.length, 4);
  assert.deepEqual(m.slots[0], slotFor(engine, n, "racing"));
  const link = viewerLink(engine, racing, m);
  assert.equal(link, "../arena.html?game=racing&seed=42&cars=cutter-5-2-2,follower-3-3-3,cutter-4-3-2,blocker-3-2-4"
    + "&looks=3F6FD8-m-h,F2C14E-p,EE7A68-4,D4A23A-c&name1=Pip&name2=Biscuit&name3=Noodle&name4=Captain%20Fuzz");
  const spec = parseRaceQuery(link.split("?")[1], raceEngine, defaultRaceBuilds());
  assert.deepEqual(spec.notes, []);
  assert.deepEqual(spec.names, ["Pip", "Biscuit", "Noodle", "Captain Fuzz"]);
  const builds = [n.builds.racing, ...racing.house.map((id) => houseById(id).builds.racing)];
  assert.deepEqual(spec.builds, builds);
  assert.deepEqual(runRace(spec.seed, spec.builds), runRace("42", builds));
  // Swapping a house Nyborg for one of yours changes who plays, through the same path.
  const juno = { ...createNyborg(engine, "Juno"), look: { hair_style: "pom", hair_color: "#C8459A", strands: 3, accessories: [] } };
  const swapped = matchFor(engine, racing, [n, houseById("house-biscuit"), juno], "7");
  assert.match(viewerLink(engine, racing, swapped), /cars=cutter-5-2-2,follower-3-3-3,follower-3-3-3&looks=3F6FD8-m-h,F2C14E-p,C8459A-p&name1=Pip&name2=Biscuit&name3=Juno$/);
});

test("tank quick match: you are Blue against Pepper; cosmetics stay out of the tank query", () => {
  const n = pip();
  const tank = GAME_MODULES.find((m) => m.id === "tank");
  const link = viewerLink(engine, tank, quickMatch(engine, tank, n));
  const q = link.split("?")[1];
  assert.equal(q, "seed=42&blue=kiter-4-3-2&orange=charger-4-1-4&looks=3F6FD8-m-h,B83246-m-g&name1=Pip&name2=Pepper");
  const plain = queryFromSpec(specFromQuery(`?${q}`, wasm.canonicalTankQuery));
  assert.ok(!plain.includes("looks") && !plain.includes("name"));
  assert.deepEqual(runTank(plain), runTank(q.split("&looks")[0]));
});

test("watch sessions: the app plays the same race and tank match as the engine, and reads the result", () => {
  const n = pip();
  for (const mod of GAME_MODULES) {
    for (const seed of ["42", "7", "18446744073709551615"]) {
      const match = quickMatch(engine, mod, n, seed);
      const s = mod.start(engine, match);
      assert.ok(s.width > 0 && s.height > 0 && !s.over() && s.result() === null);
      assert.equal(s.board().length, match.slots.length);
      let guard = 0;
      while (!s.over() && guard++ < 10_000) s.step(37); // uneven steps, like frames
      assert.ok(s.over(), `${mod.id} ${seed} ends`);
      const direct = mod.id === "racing"
        ? runRace(seed, match.slots.map((x) => x.build))[0]
        : runTank(mod.query(engine, match).split("&looks")[0])[0];
      assert.equal(s.hash(), direct, `${mod.id} seed ${seed}: same final state as a direct run`);
      const res = s.result();
      assert.deepEqual([...res.order].sort(), match.slots.map((_, i) => i), "every slot placed once");
      if (res.winner != null) { assert.equal(res.order[0], res.winner); assert.equal(res.place(res.winner), 1); }
      assert.ok(typeof res.headline === "string" && res.seconds > 0);
      for (const slot of res.order) assert.equal(typeof res.detail(slot), "string");
      s.free();
    }
  }
});

test("edit: the tank triangle maps all 19 loadouts to valid 9-point builds; other games use the pips", async () => {
  const { loadoutOf, levelsOf } = await import("../../web/games/tank.js");
  const tcat = engine.tankCatalog();
  assert.equal(tcat.loadouts.length, 19);
  for (const l of tcat.loadouts) {
    assert.equal(loadoutOf(levelsOf(l)), l);
    const b = { rules_version: 1, levels: levelsOf(l), behavior: { kind: "scripted", id: "kiter" } };
    assert.equal(engine.validateBuild("tank", b).ok, true, l);
  }
  // The triangle only ever lands on the engine's snap, which is always one of the 19.
  for (const w of [[1, 0, 0], [0, 1, 0], [0, 0, 1], [0.2, 0.3, 0.5], [0.9, 0.05, 0.05]]) assert.ok(tcat.loadouts.includes(wasm.snapLoadout(...w)));
  const tank = GAME_MODULES.find((m) => m.id === "tank"), racing = GAME_MODULES.find((m) => m.id === "racing");
  assert.equal(typeof tank.editor, "function", "tank keeps the triangle (decision 8)");
  assert.equal(racing.editor, undefined, "racing (and any new game) gets the schema-driven pips");
});

test("backup: import puts Nyborgs in the library; a clash can skip, keep both or replace", async () => {
  const { importInto } = await import("../../web/app/screens/backup.js");
  const { Library, MemoryStorage, exportAll } = await import("../../web/nyborg-library.js");
  const lib = new Library(new MemoryStorage());
  lib.load();
  const a = pip(), b = { ...createNyborg(engine, "Juno") };
  const file = exportAll([a, b]);
  let r = await importInto(engine, lib, file, () => assert.fail("no clash yet"));
  assert.deepEqual([r.added, r.error, lib.list.map((n) => n.name)], [2, null, ["Pip", "Juno"]]);
  const answers = ["both", "skip"];
  r = await importInto(engine, lib, file, async () => answers.shift());
  assert.equal(r.added, 1);
  assert.deepEqual(lib.list.map((n) => n.name), ["Pip", "Juno", "Pip"]);
  assert.notEqual(lib.list[2].id, a.id, "kept both under a new id");
  const renamed = exportAll([{ ...a, name: "Pip Two" }]);
  r = await importInto(engine, lib, renamed, async () => "replace");
  assert.equal(lib.get(a.id).name, "Pip Two");
  assert.match((await importInto(engine, lib, "nope", () => "skip")).error, /JSON/);
});

test("save from a link: parseQuery reads back what query wrote, for every arena", () => {
  const me = { ...createNyborg(engine, "Pip"), id: "ny-me", look: { hair_style: "mop", hair_color: "#3F6FD8", strands: 3, accessories: ["hat"] } };
  for (const mod of arenas(engine)) {
    const match = quickMatch(engine, mod, me, "42");
    const params = new URLSearchParams(mod.query(engine, match));
    assert.equal(gameForQuery(engine, params).id, mod.id);
    const back = mod.parseQuery(engine, params);
    assert.equal(back.seed, "42");
    assert.equal(back.slots.length, match.slots.length);
    back.slots.forEach((s, i) => {
      assert.deepEqual(s.build, match.slots[i].build, `${mod.id} slot ${i} build`);
      assert.equal(s.name, match.slots[i].name);
      assert.deepEqual(s.look, match.slots[i].look);
      assert.equal(back.fellBack[i], false);
    });
    // And re-encoding is stable, so the app's share link is the viewer's link.
    assert.equal(mod.query(engine, { ...back, seed: "42" }), mod.query(engine, match));
  }
});

test("save from a link: a bad build falls back and is flagged; bot duels are refused", () => {
  const racing = arenas(engine).find((m) => m.id === "racing");
  const r = racing.parseQuery(engine, new URLSearchParams("game=racing&seed=9&cars=cutter-9-9-9,follower-3-3-3&name1=Zed"));
  assert.deepEqual(r.fellBack, [true, false]);
  assert.equal(engine.validateBuild("racing", r.slots[0].build).ok, true);
  assert.match(r.notes.join(" "), /Car 1/);
  const tank = arenas(engine).find((m) => m.id === "tank");
  assert.throws(() => tank.parseQuery(engine, new URLSearchParams("seed=1&a=chaser&b=wanderer")), /built-in bots/);
  assert.equal(gameForQuery(engine, new URLSearchParams("seed=1")).id, "tank");
  assert.equal(gameForQuery(engine, new URLSearchParams("game=chess")), null);
});
