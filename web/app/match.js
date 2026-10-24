// A match from the hash: ?slots=<id>,<id>…&seed=N, shared by match setup and watch. Ids are your
// Nyborgs or house Nyborgs; unknown ids drop out and the lineup tops up from the house.
import { lineup, matchFor } from "../games/index.js?v=20261024b";
import { houseById, isHouse } from "./house.js?v=20261024b";

const U64_MAX = 18446744073709551615n;
export const seedOk = (s) => /^\d{1,20}$/.test(String(s)) && BigInt(s) <= U64_MAX;
// A fresh seed for "Shuffle" / "New seed": any u32 is a valid u64 seed, and the match stays
// deterministic for the seed it lands on (the seed is in the hash and the share link).
export const freshSeed = () => String(crypto.getRandomValues(new Uint32Array(1))[0]);

export function resolveMatch(app, mod, params) {
  const who = (id) => (isHouse(id) ? houseById(id) : app.lib.get(id));
  const asked = params.get("me");
  const me = asked && app.lib.get(asked) ? asked : app.lib.selected || app.lib.list[0]?.id;
  const ids = lineup(mod, who, me, (params.get("slots") || "").split(",").filter(Boolean));
  const seed = seedOk(params.get("seed") || "") ? params.get("seed") : "42";
  return { ids, seed, who, match: () => matchFor(app.engine, mod, ids.map(who), seed) };
}

/** The hash query for a lineup and seed. */
export const matchQuery = (ids, seed) => `slots=${ids.join(",")}&seed=${seed}`;
