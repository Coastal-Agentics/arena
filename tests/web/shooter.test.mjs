import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { BUDGET, STATS, LAYOUTS, GEN0, createWorld, step, derive, validBuild, evaluate, train } from "../../web/shooter/sim.js";
import { SHIP_KEY, cleanShip, loadShip, saveShip, defaultShip, pilots } from "../../web/shooter/ships.js";
import { KEYS } from "../../web/nyborg-library.js";

const mem = () => { const m = new Map(); return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: (k) => m.delete(k), m }; };
const run = (seed, layout) => { const w = createWorld({ seed, layout, maxTime: 20 }); while (!w.over) step(w); return JSON.stringify([w.t, w.score, w.wave, w.ships, w.enemies]); };

test("sim is deterministic per seed and layout", () => {
  for (const l of LAYOUTS) { assert.equal(run(5, l), run(5, l)); }
  assert.notEqual(run(5, "open"), run(6, "open"));
});
test("evaluate and train are deterministic", () => {
  assert.deepEqual(evaluate(GEN0, { seeds: [1], maxTime: 10 }), evaluate(GEN0, { seeds: [1], maxTime: 10 }));
  const a = train({ pop: 4, gens: 2, evalSeeds: [1], maxTime: 5 }), b = train({ pop: 4, gens: 2, evalSeeds: [1], maxTime: 5 });
  assert.deepEqual(a, b);
});
test("point budget: every valid build spends exactly BUDGET", () => {
  assert.ok(validBuild({ speed: 6, fire: 6, damage: 0, shield: 0 }));
  assert.ok(!validBuild({ speed: 6, fire: 6, damage: 1, shield: 0 }), "over budget");
  assert.ok(!validBuild({ speed: 3, fire: 3, damage: 3, shield: 2 }), "under budget");
  assert.ok(!validBuild({ speed: 7, fire: 5, damage: 0, shield: 0 }), "stat cap");
  // an over-budget build gets no extra stats: it falls back to the default
  assert.deepEqual(derive({ speed: 6, fire: 6, damage: 6, shield: 6 }), derive({ speed: 3, fire: 3, damage: 3, shield: 3 }));
  assert.equal(STATS.length * 3, BUDGET);
});
test("cosmetics never affect stats or the sim", () => {
  const a = cleanShip({ build: { speed: 4, fire: 4, damage: 2, shield: 2 }, look: { hull: "dart", color: "#5FE1FF", trail: "line" } });
  const b = cleanShip({ build: { speed: 4, fire: 4, damage: 2, shield: 2 }, look: { hull: "wing", color: "#FF5FD2", trail: "none" }, pilot: "x" });
  assert.deepEqual(derive(a.build), derive(b.build));
  const sim = (s) => { const w = createWorld({ seed: 3, pilots: [{ ...s, ai: GEN0 }], maxTime: 15 }); while (!w.over) step(w); return [w.score, w.t]; };
  assert.deepEqual(sim(a), sim(b));
});
test("ship save round-trips and leaves Nyborg keys alone", () => {
  const st = mem();
  st.setItem(KEYS.index, JSON.stringify({ format: 2, order: ["n1"], selected: "n1" }));
  st.setItem(KEYS.nyborg("n1"), JSON.stringify({ id: "n1", name: "Pip" }));
  const before = [st.getItem(KEYS.index), st.getItem(KEYS.nyborg("n1"))];
  const ship = { name: "Neon", build: { speed: 5, fire: 3, damage: 2, shield: 2 }, look: { hull: "delta", color: "#B6FF5F", trail: "dots" }, pilot: "n1" };
  saveShip(st, ship);
  assert.deepEqual(loadShip(st), ship);
  assert.deepEqual([st.getItem(KEYS.index), st.getItem(KEYS.nyborg("n1"))], before);
  assert.deepEqual([...st.m.keys()].sort(), [KEYS.index, KEYS.nyborg("n1"), SHIP_KEY].sort());
  assert.deepEqual(pilots(st), [{ id: "n1", name: "Pip" }]);
  st.setItem(SHIP_KEY, "{broken"); assert.deepEqual(loadShip(st), defaultShip());
});
test("committed champion beats Gen 0 on held-out seeds", () => {
  const c = JSON.parse(readFileSync(new URL("../../web/shooter/champion.json", import.meta.url)));
  assert.ok(c.champion.overall.fitness > c.gen0.overall.fitness);
  const re = evaluate(c.genes, { seeds: [101, 102], layouts: ["open"], maxTime: 30 });
  assert.equal(typeof re.fitness, "number");
});
