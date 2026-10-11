// Tank Arena, as an app game module (docs/design/nyborg-app.md §6): its slots, its house
// opponent and how a match link is written. Links are exactly the viewer's tank format:
// ?seed=S&blue=<behavior>-<a>-<s>-<d>&orange=…, then looks and names for the drawing only.
import { encodeBuild, cosmeticsQuery } from "../nyborg-link.js";

export default {
  id: "tank",
  title: "Tank Arena",
  button: "Tank ▶",
  start: "Start the match ▶",
  setup: "Pick your opponent",
  blurb: "Two Nyborgs, two tanks, one arena. Last tank standing wins.",
  slots: { min: 2, max: 2, names: ["Blue", "Orange"] },
  /** The house Nyborg (app/house.js) you face by default. */
  house: ["house-pepper"],

  /** The viewer query for {seed, slots: [{build, look, name}]}. */
  query(engine, { seed, slots }) {
    const part = (s) => encodeBuild(engine.catalog("tank"), s.build);
    const q = engine.canonicalTankQuery(`seed=${seed}&blue=${part(slots[0])}&orange=${part(slots[1])}`);
    const cos = cosmeticsQuery(slots.map((s) => ({ look: s.look, name: s.name })));
    return `${q}${cos ? `&${cos}` : ""}`;
  },
};
