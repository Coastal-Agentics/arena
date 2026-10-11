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
import { GAME_MODULES, arenas, slotFor, quickMatch, viewerLink } from "../../web/games/index.js";
import { createNyborg } from "../../web/nyborg-library.js";
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
  for (const m of GAME_MODULES) for (const k of ["id", "title", "button", "slots", "house", "query"]) assert.ok(k in m, `${m.id}.${k}`);
  const third = { id: "golf", title: "Golf", button: "Golf ▶", slots: { min: 1, max: 1, names: ["You"] }, house: () => [], query: () => "game=golf" };
  const fakeEngine = { ...engine, games: () => [...engine.games(), { game: "golf", rules_version: 1 }] };
  assert.deepEqual(arenas(fakeEngine, [...GAME_MODULES, third]).map((m) => m.id), ["racing", "tank", "golf"]);
  assert.deepEqual(arenas(engine, [...GAME_MODULES, third]).map((m) => m.id), ["racing", "tank"], "no engine game, no arena");
});

test("racing quick match: you plus the house grid; the link plays the direct race", () => {
  const n = pip();
  const racing = GAME_MODULES.find((m) => m.id === "racing");
  const m = quickMatch(engine, racing, n);
  assert.equal(m.slots.length, 4);
  assert.deepEqual(m.slots[0], slotFor(engine, n, "racing"));
  const link = viewerLink(engine, racing, m);
  assert.equal(link, "../arena.html?game=racing&seed=42&cars=cutter-5-2-2,cutter-3-3-3,blocker-3-3-3,follower-3-3-3&looks=3F6FD8-m-h,,,&name1=Pip");
  const spec = parseRaceQuery(link.split("?")[1], raceEngine, defaultRaceBuilds());
  assert.deepEqual(spec.notes, []);
  assert.deepEqual(spec.builds, [n.builds.racing, ...defaultRaceBuilds().slice(1)]);
  assert.deepEqual(runRace(spec.seed, spec.builds), runRace("42", [n.builds.racing, ...defaultRaceBuilds().slice(1)]));
});

test("tank quick match: you are Blue against the house; cosmetics stay out of the tank query", () => {
  const n = pip();
  const tank = GAME_MODULES.find((m) => m.id === "tank");
  const link = viewerLink(engine, tank, quickMatch(engine, tank, n));
  const q = link.split("?")[1];
  assert.match(q, /^seed=42&blue=kiter-4-3-2&orange=[a-z]+-\d-\d-\d&looks=3F6FD8-m-h,&name1=Pip$/);
  const plain = queryFromSpec(specFromQuery(`?${q}`, wasm.canonicalTankQuery));
  assert.ok(!plain.includes("looks") && !plain.includes("name"));
  assert.deepEqual(runTank(plain), runTank(q.split("&looks")[0]));
});
