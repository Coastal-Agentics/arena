// Tank Arena, as an app game module (docs/design/nyborg-app.md §6): its slots, its house
// opponent and how a match link is written. Links are exactly the viewer's tank format:
// ?seed=S&blue=<behavior>-<a>-<s>-<d>&orange=…, then looks and names for the drawing only.
import { encodeBuild, decodeBuild, cosmeticsQuery } from "../nyborg-link.js";

export default {
  id: "tank",
  title: "Tank Arena",
  button: "Tank ▶",
  blurb: "Two Nyborgs, two tanks, one arena.",
  slots: { min: 2, max: 2, names: ["Blue", "Orange"] },

  /** The house side for slots after yours: today's default orange tank (no look, no name). */
  house(engine) {
    const q = new URLSearchParams(engine.tankCatalog().default_query);
    const b = decodeBuild(engine.catalog("tank"), q.get("orange"));
    return [{ build: b, look: null, name: null, house: true }];
  },

  /** The viewer query for {seed, slots: [{build, look, name}]}. */
  query(engine, { seed, slots }) {
    const part = (s) => encodeBuild(engine.catalog("tank"), s.build);
    const q = engine.canonicalTankQuery(`seed=${seed}&blue=${part(slots[0])}&orange=${part(slots[1])}`);
    const cos = cosmeticsQuery(slots.map((s) => ({ look: s.look, name: s.name })));
    return `${q}${cos ? `&${cos}` : ""}`;
  },
};
