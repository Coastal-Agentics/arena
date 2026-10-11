// Racing on the Ring, as an app game module (docs/design/nyborg-app.md §6). Links are the
// viewer's racing format (nyborg-link.js): ?game=racing&seed=S&cars=…&looks=…&nameN=….
import { raceQuery } from "../nyborg-link.js";
import { defaultRaceBuilds } from "../race-ui.js";

export default {
  id: "racing",
  title: "Racing",
  button: "Race ▶",
  blurb: "Up to four Nyborgs, three laps of the Ring.",
  slots: { min: 2, max: 4, names: ["Car 1", "Car 2", "Car 3", "Car 4"] },

  /** The house cars that fill the grid after yours: today's default drivers. */
  house() {
    return defaultRaceBuilds().slice(1).map((build) => ({ build, look: null, name: null, house: true }));
  },

  query(engine, { seed, slots }) {
    return raceQuery(engine.catalog("racing"), {
      seed, custom: true,
      builds: slots.map((s) => s.build), looks: slots.map((s) => s.look), names: slots.map((s) => s.name),
    });
  },
};
