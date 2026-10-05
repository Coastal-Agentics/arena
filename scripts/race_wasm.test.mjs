// WasmRace and checkRaceReplayJson in the committed web/pkg:
//   node --test scripts/race_wasm.test.mjs
// CI runs it through scripts/catalog_racing.test.mjs, which imports this file.
// - the same seed and builds give the native final_hash (pinned in
//   engine-py/tests/fixtures/determinism.json by engine-py's native Rust test);
// - racing replays written from Python (engine-py's test_determinism.py with
//   SALTMARSH_ARENA_REPLAY_DIR set; scripts/fixtures/python-racing/) verify in wasm.
// The same cases run natively in engine-wasm's race tests (cargo test). No npm deps.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const path = (p) => new URL(p, import.meta.url);
const m = await import("../web/pkg/engine_wasm.js");
m.initSync({ module: readFileSync(path("../web/pkg/engine_wasm_bg.wasm")) });

const cases = JSON.parse(
  readFileSync(path("../engine-py/tests/fixtures/determinism.json"), "utf8"),
).cases;
const caseNamed = (name) => cases.find((c) => c.name === name);
const runToEnd = (r) => {
  while (!r.step(64));
  return r;
};

test("an all-scripted race gives the native final_hash, outcome and ticks", () => {
  const c = caseNamed("racing-all-scripted");
  const r = runToEnd(m.WasmRace.fromBuilds(String(c.seed), JSON.stringify(c.builds)));
  assert.equal(r.stateHash(), c.final_hash);
  assert.equal(r.tick(), c.ticks);
  assert.equal(r.isOver(), true);
  const o = JSON.parse(r.outcomeJson());
  assert.deepEqual(
    { winner: o.winner, ticks: o.ticks, reason: o.reason },
    c.outcome,
  );
  const replay = JSON.parse(r.replayJson());
  assert.equal(replay.format, 5);
  assert.equal(replay.game, "racing");
  assert.equal(replay.final_hash, c.final_hash);
  // The Python run of the same race wrote the same replay, byte for byte.
  const py = readFileSync(path("fixtures/python-racing/racing-all-scripted.json"), "utf8");
  assert.equal(r.replayJson(), py);
  r.free();
});

for (const name of ["racing-all-scripted", "racing-solo"]) {
  test(`Python-written racing replay ${name} verifies in wasm`, () => {
    const c = caseNamed(name);
    const json = readFileSync(path(`fixtures/python-racing/${name}.json`), "utf8");
    const check = JSON.parse(m.checkRaceReplayJson(json));
    assert.equal(check.verify_error, null);
    assert.equal(check.final_hash, c.final_hash);
    assert.equal(check.game, "racing");
    assert.equal(check.format, 5);
    // Seeds are u64 decimal strings (2^64-1 here; a JS number would round it).
    assert.equal(check.seed, json.match(/"seed":"(\d+)"/)[1]);
    assert.equal(check.ticks, c.ticks);
    assert.equal(check.cars, c.builds.length);
    const o = check.outcome;
    assert.deepEqual({ winner: o.winner, ticks: o.ticks, reason: o.reason }, c.outcome);
    // A tampered hash re-simulates and reports the mismatch.
    const bad = JSON.parse(
      m.checkRaceReplayJson(json.replace(c.final_hash, "0000000000000000")),
    );
    assert.equal(bad.final_hash, c.final_hash);
    assert.notEqual(bad.verify_error, null);
  });
}

test("checkRaceReplayJson throws on a tank replay; checkReplayJson on a racing one", () => {
  const json = readFileSync(path("fixtures/python-racing/racing-solo.json"), "utf8");
  assert.throws(() => m.checkReplayJson(json));
  const tank = readFileSync(path("../engine-wasm/tests/parity/cw-seed-max.json"), "utf8");
  assert.equal(JSON.parse(m.checkReplayJson(tank)).verify_error, null);
  assert.throws(() => m.checkRaceReplayJson(tank));
});

test("stateJson and trackJson shapes", () => {
  const c = caseNamed("racing-four-learning");
  const r = m.WasmRace.fromBuilds("42", JSON.stringify(c.builds));
  const t = JSON.parse(r.trackJson());
  assert.equal(t.name, "Ring");
  assert.deepEqual([t.width, t.height, t.tick_hz, t.start_finish_gate], [800, 600, 60, 0]);
  assert.equal(t.gates.length, t.centreline.length);
  assert.equal(t.inner.length, t.centreline.length);
  assert.equal(t.outer.length, t.centreline.length);
  assert.deepEqual(t.gates.map((g) => g.start_finish), t.gates.map((_, i) => i === 0));
  assert.ok(t.half_width > 0 && t.car_radius > 0 && t.laps > 0);
  let s = JSON.parse(r.stateJson());
  assert.deepEqual([s.tick, s.over, s.outcome], [0, false, null]);
  assert.deepEqual(s.cars.map((car) => car.id), [0, 1, 2, 3]);
  assert.deepEqual(Object.keys(s.cars[0]), [
    "id", "pos", "heading", "speed", "lap", "laps_total", "next_gate", "started",
    "finished", "finish_tick", "placing", "slot",
  ]);
  assert.equal(r.step(30), false);
  assert.equal(r.tick(), 30);
  assert.equal(r.outcomeJson(), "null");
  runToEnd(r);
  s = JSON.parse(r.stateJson());
  assert.equal(s.over, true);
  assert.deepEqual([...s.outcome.placings].sort(), [1, 2, 3, 4]);
  assert.deepEqual(s.outcome, JSON.parse(r.outcomeJson()));
  assert.deepEqual(
    JSON.parse(r.setupJson()).cars.map((car) => car.behavior),
    c.builds.map((b) => b.behavior.id),
  );
  r.free();
});

test("fromBuilds rejects bad input like WasmMatch does", () => {
  const ok = JSON.stringify(caseNamed("racing-solo").builds);
  const one = caseNamed("racing-solo").builds[0];
  const throwsWith = (seed, builds, re) =>
    assert.throws(() => m.WasmRace.fromBuilds(seed, builds), re);
  throwsWith("1", "[]", /a race takes 1 to 4 builds/);
  throwsWith("1", JSON.stringify(Array(5).fill(one)), /got 5/);
  throwsWith("-1", ok, /seed/);
  throwsWith(
    "1",
    JSON.stringify([one, { ...one, levels: { power: 5, top_speed: 5, grip: 5 } }]),
    /^Error: car 1: invalid build \[\{"code":"over_budget","key":"levels"\}\]$/,
  );
  throwsWith(
    "1",
    JSON.stringify([{ ...one, behavior: { kind: "champion", ref: "racing/nightly/gen-1" } }]),
    /car 0: a champion behavior/,
  );
});
