// In-browser "Train": the same genetic search as train.mjs, off the main thread.
import { GEN0, evaluate, train } from "./sim.js";
onmessage = ({ data }) => {
  const { seed = 7, pop = 16, gens = 10 } = data || {};
  const champ = train({ seed, pop, gens, evalSeeds: [1, 2], maxTime: 40, onGen: (p) => postMessage({ type: "gen", ...p, gens }) });
  const held = { seeds: [101, 102, 103], maxTime: 60 };
  postMessage({ type: "done", genes: champ.genes, gen0: evaluate(GEN0, held), champion: evaluate(champ.genes, held) });
};
