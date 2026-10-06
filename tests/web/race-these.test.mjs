// Node tests for "Race these": racing links that carry Nyborgs (web/nyborg-link.js).
//   node --test "tests/web/**/*.test.mjs"
// URL encode/decode round trips, the plain link staying byte-identical, the invalid-build
// fallback, and the race a link plays equalling a direct WasmRace.fromBuilds run.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync, catalogJson, defaultBuild, validateBuild, canonicalTankQuery, WasmRace } from "../../web/pkg/engine_wasm.js";
import {
  encodeBuild, decodeBuild, encodeLook, decodeLook, cleanLinkName, cosmeticsQuery, parseCosmetics, raceQuery, parseRaceQuery, MAX_CARS,
} from "../../web/nyborg-link.js";
import { raceSpecFromQuery, queryFromRaceSpec, defaultRaceBuilds, lookForCar, RACE_HAIR } from "../../web/race-ui.js";
import { specFromQuery } from "../../web/tank-ui.js";
import { HAIR_PALETTE } from "../../web/nyborg-draw.js";

const root = new URL("../../", import.meta.url);
initSync({ module: readFileSync(fileURLToPath(new URL("web/pkg/engine_wasm_bg.wasm", root))) });

const RACING = JSON.parse(catalogJson("racing"));
const engine = {
  catalog: RACING,
  defaultBuild: () => JSON.parse(defaultBuild("racing")),
  validateBuild: (b) => JSON.parse(validateBuild("racing", JSON.stringify(b))),
};
const build = (id, p, t, g) => ({ rules_version: 1, levels: { power: p, top_speed: t, grip: g }, behavior: { kind: "scripted", id } });

/** Every valid racing build: 19 setups (1–5 each, 9 points) × the catalog's behaviors. */
function allValid() {
  const out = [];
  for (let p = 1; p <= 5; p++) for (let t = 1; t <= 5; t++) {
    const g = 9 - p - t;
    if (g >= 1 && g <= 5) for (const id of RACING.behaviors) out.push(build(id, p, t, g));
  }
  return out;
}

function runToEnd(seed, builds) {
  const r = WasmRace.fromBuilds(seed, JSON.stringify(builds));
  r.step(1_000_000);
  const out = { outcome: r.outcomeJson(), hash: r.stateHash(), setup: r.setupJson() };
  r.free();
  return out;
}

test("builds: encode/decode round trip for every valid racing build", () => {
  const all = allValid();
  assert.equal(all.length, 19 * RACING.behaviors.length);
  for (const b of all) {
    const token = encodeBuild(RACING, b);
    assert.match(token, /^[a-z]+-[1-5]-[1-5]-[1-5]$/);
    assert.deepEqual(decodeBuild(RACING, token), b);
    assert.ok(engine.validateBuild(b).ok);
  }
  assert.equal(encodeBuild(RACING, build("cutter", 5, 2, 2)), "cutter-5-2-2");
  for (const bad of ["", "cutter", "cutter-5-2", "cutter-5-2-2-1", "Cutter-5-2-2", "cutter-a-2-2", "cutter-5--2-2", "-5-2-2", "cutter-5-2-2.0", "cutter-123-1-1"]) {
    assert.equal(decodeBuild(RACING, bad), null, bad);
  }
});

test("looks: encode/decode round trip for every palette colour, strand count and accessory set", () => {
  let n = 0;
  for (const { hex } of HAIR_PALETTE) for (const strands of [2, 3, 4]) for (const accessories of [[], ["glasses"], ["hat"], ["glasses", "hat"]]) {
    const look = { hair_color: hex, strands, accessories };
    assert.deepEqual(decodeLook(encodeLook(look)), look);
    n++;
  }
  assert.equal(n, 120);
  assert.equal(encodeLook({ hair_color: "#D9534F", strands: 3, accessories: ["hat", "glasses"] }), "D9534F-3-gh");
  assert.equal(encodeLook({ hair_color: "#3f6fd8", strands: 2, accessories: [] }), "3F6FD8-2");
  assert.deepEqual(decodeLook("d9534f-4-hg"), { hair_color: "#D9534F", strands: 4, accessories: ["glasses", "hat"] });
  for (const bad of ["00FF00-3", "D9534F-5", "D9534F-1", "D9534F-3-gg", "D9534F-3-x", "#D9534F-3", "D9534F", "<b>-3"]) {
    assert.equal(decodeLook(bad), null, bad);
  }
  assert.equal(cleanLinkName("  Pip  "), "Pip");
  assert.equal(cleanLinkName("a\u0000b"), "ab");
  assert.equal(cleanLinkName(""), null);
  assert.equal(cleanLinkName("x".repeat(25)), null);
});

test("racing link: full round trip with cars, looks and names", () => {
  const spec = {
    seed: "7",
    builds: [build("cutter", 5, 2, 2), build("follower", 2, 2, 5), build("cutter", 5, 2, 2)],
    custom: true,
    looks: [{ hair_color: "#D9534F", strands: 3, accessories: ["glasses"] }, null, { hair_color: "#F2C14E", strands: 4, accessories: ["hat"] }],
    names: ["Pip", "Juno & co, <b>", "Pip 2 🧶"],
  };
  const q = raceQuery(RACING, spec);
  assert.equal(q, "game=racing&seed=7&cars=cutter-5-2-2,follower-2-2-5,cutter-5-2-2&looks=D9534F-3-g,,F2C14E-4-h"
    + "&name1=Pip&name2=Juno%20%26%20co%2C%20%3Cb%3E&name3=Pip%202%20%F0%9F%A7%B6");
  const back = parseRaceQuery("?" + q, engine, defaultRaceBuilds());
  assert.deepEqual(back, { game: "racing", ...spec, notes: [] });
  assert.equal(raceQuery(RACING, back), q, "canonical: parse → print is stable");
  // The viewer's own wrappers give the same thing.
  assert.deepEqual(raceSpecFromQuery(q, engine), back);
  assert.equal(queryFromRaceSpec(back, { speed: 2, paused: true }, RACING), q + "&speed=2&paused=1");
  assert.deepEqual(lookForCar(back, 0), { hair: "#D9534F", strands: 3, accessories: ["glasses"] });
  assert.deepEqual(lookForCar(back, 1), { hair: RACE_HAIR[1], strands: 3, accessories: [] }, "no look: default palette");
  // One car is allowed; names on slots beyond the cars are ignored.
  const one = parseRaceQuery("game=racing&seed=1&cars=blocker-1-3-5&name1=Solo&name3=Ghost", engine, defaultRaceBuilds());
  assert.deepEqual([one.builds.length, one.names, one.notes], [1, ["Solo"], []]);
});

test("no link params: exactly today's race and URL", () => {
  for (const search of ["?game=racing&seed=42", "game=racing&seed=42", "?game=racing", "?game=racing&seed=42&speed=2"]) {
    const s = parseRaceQuery(search, engine, defaultRaceBuilds());
    assert.equal(s.custom, false);
    assert.deepEqual(s.builds, defaultRaceBuilds());
    assert.deepEqual(s.notes, []);
    assert.equal(s.seed, "42");
    assert.equal(queryFromRaceSpec(s, {}, RACING), "game=racing&seed=42");
  }
  // The old no-engine path still works and prints the same.
  const old = raceSpecFromQuery("?game=racing&seed=9");
  assert.equal(queryFromRaceSpec(old, { speed: 4, paused: 1, t: 600 }), "game=racing&seed=9&speed=4&paused=1&t=600");
  assert.deepEqual(old.builds, defaultRaceBuilds());
  // Looks without cars dress the default grid; still no `cars` in the URL.
  const dressed = parseRaceQuery("?game=racing&seed=42&looks=C8459A-2&name1=Pip", engine, defaultRaceBuilds());
  assert.equal(queryFromRaceSpec(dressed, {}, RACING), "game=racing&seed=42&looks=C8459A-2,,,&name1=Pip");
  assert.equal(runToEnd("42", dressed.builds).hash, runToEnd("42", defaultRaceBuilds()).hash);
});

test("invalid builds fall back to the default build, with a note each", () => {
  const def = engine.defaultBuild();
  assert.deepEqual(def, build("follower", 3, 3, 3));
  const s = parseRaceQuery("?game=racing&seed=3&cars=cutter-5-5-5,zzz,blocker-2-3-4,flyer-3-3-3,cutter-0-4-5", engine, defaultRaceBuilds());
  assert.equal(s.custom, true);
  assert.deepEqual(s.builds, [def, def, build("blocker", 2, 3, 4), def], "five cars: only the first four race");
  assert.equal(s.notes.length, 4);
  assert.match(s.notes[0], /first 4 race/);
  assert.match(s.notes[1], /^Car 1 \(cutter-5-5-5\) isn't a valid build: That's more than 9 points.*default build instead\.$/);
  assert.match(s.notes[2], /^Car 2 \(zzz\) isn't a valid build: It isn't written like/);
  assert.match(s.notes[3], /^Car 4 \(flyer-3-3-3\).*default build instead\.$/);
  // The fallback is printed back as what actually races.
  assert.equal(queryFromRaceSpec(s, {}, RACING), "game=racing&seed=3&cars=follower-3-3-3,follower-3-3-3,blocker-2-3-4,follower-3-3-3");
  // Under budget, out of range, an empty slot.
  const u = parseRaceQuery("cars=cutter-1-1-1,blocker-6-2-1,", engine, defaultRaceBuilds());
  assert.deepEqual(u.builds, [def, def, def]);
  assert.match(u.notes[0], /points left to spend/);
  assert.match(u.notes[2], /Car 3 \(empty\)/);
  // No cars at all: the default race.
  const none = parseRaceQuery("cars=", engine, defaultRaceBuilds());
  assert.deepEqual([none.custom, none.builds, none.notes.length], [false, defaultRaceBuilds(), 1]);
  // An unreadable look keeps that car's default colours, with a note.
  const lk = parseRaceQuery("cars=cutter-5-2-2,cutter-5-2-2&looks=00FF00-3,D9534F-9", engine, defaultRaceBuilds());
  assert.deepEqual(lk.looks, [null, null]);
  assert.equal(lk.notes.length, 2);
  assert.ok(MAX_CARS === 4);
});

test("a link's race equals a direct WasmRace.fromBuilds run; cosmetics never change it", () => {
  const cases = [
    ["42", [build("cutter", 5, 2, 2), build("follower", 2, 2, 5)]],
    ["7", [build("blocker", 3, 3, 3), build("cutter", 4, 3, 2), build("follower", 1, 5, 3), build("cutter", 4, 3, 2)]],
    ["18446744073709551615", [build("follower", 5, 3, 1)]],
  ];
  for (const [seed, builds] of cases) {
    const direct = runToEnd(seed, builds);
    const looks = builds.map((_, i) => ({ hair_color: HAIR_PALETTE[i * 3].hex, strands: 2 + (i % 3), accessories: i % 2 ? ["hat"] : ["glasses"] }));
    const plain = raceQuery(RACING, { seed, builds, custom: true });
    const dressed = raceQuery(RACING, { seed, builds, custom: true, looks, names: builds.map((_, i) => `Nyborg ${i + 1}`) });
    for (const q of [plain, dressed]) {
      const s = raceSpecFromQuery(q, engine);
      const viaLink = runToEnd(s.seed, s.builds);
      assert.deepEqual(viaLink, direct, `${seed}: ${q}`);
    }
    assert.ok(JSON.parse(direct.outcome).ticks > 0);
  }
  // The default grid is unchanged from before this slice.
  assert.equal(runToEnd("42", raceSpecFromQuery("?game=racing&seed=42", engine).builds).hash, runToEnd("42", defaultRaceBuilds()).hash);
});

test("tank links: looks and names ride along without touching the tank query", () => {
  const cos = cosmeticsQuery([{ look: { hair_color: "#C8459A", strands: 4, accessories: ["hat"] }, name: "Pip" }, { look: null, name: "Juno" }]);
  assert.equal(cos, "looks=C8459A-4-h,&name1=Pip&name2=Juno");
  const base = "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4";
  assert.deepEqual(specFromQuery(`?${base}&${cos}`, canonicalTankQuery), specFromQuery(`?${base}`, canonicalTankQuery));
  const back = parseCosmetics(`?${base}&${cos}`, 2);
  assert.deepEqual(back, { looks: [{ hair_color: "#C8459A", strands: 4, accessories: ["hat"] }, null], names: ["Pip", "Juno"], notes: [] });
  assert.equal(cosmeticsQuery([{}, {}]), "", "nothing to carry: the canonical tank link is unchanged");
});
