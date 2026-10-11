// Racing on the Ring, as an app game module (docs/design/nyborg-app.md §6). Links are the
// viewer's racing format (nyborg-link.js): ?game=racing&seed=S&cars=…&looks=…&nameN=….
import { raceQuery } from "../nyborg-link.js";

export default {
  id: "racing",
  title: "Racing",
  button: "Race ▶",
  start: "Start the race ▶",
  setup: "Set up the grid",
  blurb: "Up to four Nyborgs, three laps of the Ring. Finished Nyborgs coast as ghosts.",
  slots: { min: 2, max: 4, names: ["Car 1", "Car 2", "Car 3", "Car 4"] },
  /** House Nyborgs (app/house.js) after yours on the default grid: a follower, a cutter and a blocker. */
  house: ["house-biscuit", "house-noodle", "house-captain-fuzz"],

  query(engine, { seed, slots }) {
    return raceQuery(engine.catalog("racing"), {
      seed, custom: true,
      builds: slots.map((s) => s.build), looks: slots.map((s) => s.look), names: slots.map((s) => s.name),
    });
  },
};
