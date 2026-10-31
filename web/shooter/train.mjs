// Offline trainer (CPU only): node web/shooter/train.mjs [--gens 16] [--pop 24] [--seed 7]
// (--time S: seconds per training run, --out FILE: write elsewhere)
// Runs the genetic search on training seeds, then scores Gen 0 and the champion on held-out
// seeds and writes champion.json next to this file. Same seed + args → same file.
import { writeFileSync } from "node:fs";
import { GEN0, LAYOUTS, evaluate, train } from "./sim.js";

const arg = (k, d) => { const i = process.argv.indexOf(`--${k}`); return i > 0 ? Number(process.argv[i + 1]) : d; };
const opts = { seed: arg("seed", 7), pop: arg("pop", 24), gens: arg("gens", 20), evalSeeds: [1, 2, 3, 4], maxTime: arg("time", 90) };
const t0 = Date.now();
const champ = train({ ...opts, onGen: ({ gen, best, mean }) => console.log(`gen ${String(gen).padStart(2)}  best ${best.toFixed(1)}  mean ${mean.toFixed(1)}`) });
const HELD = [101, 102, 103, 104, 105, 106, 107, 108, 109, 110];
const round = (o) => Object.fromEntries(Object.entries(o).map(([k, v]) => [k, Math.round(v * 1000) / 1000]));
const perLayout = (g) => Object.fromEntries(LAYOUTS.map((l) => [l, round(evaluate(g, { seeds: HELD, layouts: [l], maxTime: 90 }))]));
const out = {
  format: 1,
  trained: { ...opts, heldOutSeeds: HELD, heldOutMaxTime: 90 },
  genes: round(champ.genes),
  gen0: { genes: GEN0, overall: round(evaluate(GEN0, { seeds: HELD, maxTime: 90 })), layouts: perLayout(GEN0) },
  champion: { overall: round(evaluate(champ.genes, { seeds: HELD, maxTime: 90 })), layouts: perLayout(champ.genes) },
};
writeFileSync(process.argv.includes("--out") ? process.argv[process.argv.indexOf("--out") + 1] : new URL("./champion.json", import.meta.url), JSON.stringify(out, null, 2) + "\n");
console.log("gen0     ", out.gen0.overall);
console.log("champion ", out.champion.overall);
console.log(`wrote champion.json in ${((Date.now() - t0) / 1000).toFixed(1)} s`);
