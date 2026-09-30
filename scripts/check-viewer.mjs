// Node check for the viewer's pure helpers (web/tank-ui.js) against the committed wasm
// (web/pkg): URL round-trips, triangle snapping, readouts. No browser, no dependencies.
//   node scripts/check-viewer.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { initSync, tankCatalogJson, snapLoadout, canonicalTankQuery, WasmMatch } from "../web/pkg/engine_wasm.js";
import * as ui from "../web/tank-ui.js";

const wasm = readFileSync(fileURLToPath(new URL("../web/pkg/engine_wasm_bg.wasm", import.meta.url)));
initSync({ module: wasm });
const catalog = JSON.parse(tankCatalogJson());
let checks = 0;
const ok = (c, msg) => { assert.ok(c, msg); checks++; };
const eq = (a, b, msg) => { assert.deepEqual(a, b, msg); checks++; };

// Catalog is the spec.
eq(catalog.loadouts.length, 19, "19 loadouts");
eq(catalog.presets, [["Balanced", "3-3-3"], ["Glass Cannon", "5-3-1"], ["Brawler", "4-1-4"], ["Scout", "2-5-2"]]);
eq(catalog.behaviors.map((b) => b[0]), ["charger", "kiter", "sniper"]);

// URL: spec example round-trips; every behavior x loadout round-trips through the page helpers.
const example = "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4";
const s = ui.specFromQuery("?" + example, canonicalTankQuery);
eq(ui.queryFromSpec(s), example, "spec example round-trips");
eq(ui.queryFromSpec(s, { tab: "customize" }), example + "&tab=customize");
for (const [b] of catalog.behaviors) for (const l of catalog.loadouts) {
  const q = `seed=18446744073709551615&blue=${b}-${l}&orange=sniper-${l}`;
  eq(ui.queryFromSpec(ui.specFromQuery(q, canonicalTankQuery)), q);
}
eq(ui.queryFromSpec(ui.specFromQuery("?blue=Kiter&speed=4", canonicalTankQuery)), "seed=42&blue=kiter-3-3-3&orange=charger-3-3-3", "defaults + canonical case");
eq(ui.specFromQuery("?seed=7&a=wanderer&b=Chaser", canonicalTankQuery), { mode: "bots", seed: "7", bots: ["Wanderer", "Chaser"] }, "legacy bot links");
for (const bad of ["blue=kiter-5-3-2", "seed=-1", "orange=tank"]) assert.throws(() => ui.specFromQuery("?" + bad, canonicalTankQuery), bad), checks++;

// URL round-trip reproduces the match exactly (same final state hash).
const run = (q) => { const m = WasmMatch.tank(q); while (!m.step(500)); const h = m.stateHash(); m.free(); return h; };
const again = ui.queryFromSpec(ui.specFromQuery("?" + example, canonicalTankQuery));
eq(run(example), run(again), "round-trip replays exactly");
ok(run(example) !== run("seed=42&blue=kiter-5-3-1&orange=charger-3-2-4"), "loadout change alters the sim");

// Triangle: geometry inverts; corners/centre/each loadout point snap per spec; a dense grid
// (including points outside the triangle) only ever yields the 19, and reaches all 19.
for (const l of catalog.loadouts) {
  const w = ui.loadoutWeights(l);
  const [x, y] = ui.weightsToPoint(w);
  const back = ui.pointToWeights(x, y);
  back.forEach((v, i) => ok(Math.abs(v - w[i]) < 1e-9, `weights invert for ${l}`));
  eq(snapLoadout(...back), l, `${l} point snaps to itself`);
}
const c = ui.weightsToPoint([1 / 3, 1 / 3, 1 / 3]);
eq(snapLoadout(...ui.pointToWeights(...c)), "3-3-3", "centre");
eq(snapLoadout(...ui.pointToWeights(...ui.TRI.A)), "5-2-2", "attack corner");
eq(snapLoadout(...ui.pointToWeights(...ui.TRI.S)), "2-5-2", "speed corner");
eq(snapLoadout(...ui.pointToWeights(...ui.TRI.D)), "2-2-5", "defense corner");
const seen = new Set();
for (let x = -20; x <= 240; x += 2) for (let y = -20; y <= 234; y += 2) {
  const l = snapLoadout(...ui.pointToWeights(x, y));
  ok(catalog.loadouts.includes(l), `(${x},${y}) -> ${l}`);
  seen.add(l);
}
eq(seen.size, 19, "grid reaches all 19");

// Readout: the spec's example.
const gc = ui.readout(catalog, "5-3-1", "4-1-4"), br = ui.readout(catalog, "4-1-4", "5-3-1");
eq([gc.damage, gc.oppHp, gc.hitsToKill, gc.reloadSec], [29, 790, 28, 0.75]);
eq([br.damage, br.oppHp, br.hitsToKill, br.reloadSec], [24, 460, 20, 1.07]);
eq(ui.readout(catalog, "2-5-2", "3-3-3").reloadSec, 0.52); // 31 ticks
eq([ui.readout(catalog, "2-5-2", "3-3-3").speed, ui.readout(catalog, "2-5-2", "3-3-3").turnDegPerSec], [150, 150]); // 455 BAU/tick = 150°/s
eq(ui.presetName(catalog, "4-1-4"), "Brawler");

console.log(`check-viewer: ${checks} checks passed`);
