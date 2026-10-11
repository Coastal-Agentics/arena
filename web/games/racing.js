// Racing on the Ring, as an app game module (docs/design/nyborg-app.md §6). Links are the
// viewer's racing format (nyborg-link.js): ?game=racing&seed=S&cars=…&looks=…&nameN=….
import { raceQuery } from "../nyborg-link.js";
import { drawTrack, drawRaceCar, drawFinishBanner, lookForCar } from "../race-ui.js";
import { drawYouMarker } from "../app/marker.js";


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

  /**
   * Run a match: only the seed and the builds reach the engine (WasmRace.fromBuilds, as the
   * viewer does); names and looks are for the drawing and the board, `match.you` for the marker.
   */
  start(engine, match) {
    const race = engine.wasm.WasmRace.fromBuilds(match.seed, JSON.stringify(match.slots.map((s) => s.build)));
    const track = JSON.parse(race.trackJson());
    const setup = JSON.parse(race.setupJson());
    const hz = track.tick_hz || 60;
    const cars = setup.cars.map((c, i) => {
      const s = match.slots[i] || {};
      const look = s.look
        ? { hair: s.look.hair_color, style: s.look.hair_style, strands: s.look.strands, accessories: s.look.accessories, seed: s.id || s.name || `car-${i}` }
        : lookForCar(null, i);
      return { ...c, name: s.name || c.name, hair: look.hair, look };
    });
    const facing = new Map();
    let state = JSON.parse(race.stateJson());
    const outcome = () => state.outcome || (state.over ? JSON.parse(race.outcomeJson()) : null);
    return {
      width: track.width, height: track.height, tickHz: hz,
      get state() { return state; },
      step(n) { race.step(n); state = JSON.parse(race.stateJson()); },
      over: () => !!(state.over || state.outcome),
      draw(ctx) {
        drawTrack(ctx, track);
        for (const c of [...state.cars].sort((a, b) => Number(a.finished) - Number(b.finished))) {
          facing.set(c.id, drawRaceCar(ctx, track, c, { facingPrev: facing.get(c.id) ?? 1, tick: state.tick, look: cars[c.id]?.look }));
        }
        // "you" over your Nyborgs (match.you: slot numbers), under the finish banner.
        for (const c of state.cars) if (match.you?.includes(c.id)) drawYouMarker(ctx, c.pos.x, track.height - c.pos.y - 16, { alpha: c.finished ? 0.45 : 1 });
        const o = outcome();
        if (o) drawFinishBanner(ctx, track, { ...setup, cars }, o);
      },
      clock() {
        const laps = state.cars?.[0]?.laps_total ?? track.laps ?? 3;
        const lead = Math.max(0, ...(state.cars || []).map((c) => c.lap || 0));
        return `Lap ${Math.min(lead + 1, laps)}/${laps} · ${(state.tick / hz).toFixed(1)}s`;
      },
      /** Live standings: [{slot, name, hair, place, note}], leader first. */
      board: () => [...state.cars].sort((a, b) => (a.placing ?? 99) - (b.placing ?? 99)).map((c) => ({
        slot: c.id, name: cars[c.id].name, hair: cars[c.id].hair, place: c.placing,
        note: c.finished ? `finished ${(c.finish_tick / hz).toFixed(1)}s` : c.started ? `lap ${Math.min((c.lap || 0) + 1, c.laps_total)}` : "on the grid",
      })),
      /** At the end: {winner: slot|null, order: [slot…], headline, detail(slot), seconds}. */
      result() {
        const o = outcome();
        if (!o) return null;
        const order = cars.map((_, i) => i).sort((a, b) => (o.placings?.[a] ?? 99) - (o.placings?.[b] ?? 99));
        return {
          winner: o.winner ?? null, order, seconds: o.ticks / hz,
          place: (slot) => o.placings?.[slot] ?? null,
          time: (slot) => (o.finish_ticks?.[slot] != null ? o.finish_ticks[slot] / hz : null),
          detail: (slot) => (o.finish_ticks?.[slot] != null ? `${(o.finish_ticks[slot] / hz).toFixed(1)}s` : "did not finish"),
          headline: o.winner == null ? "Race over" : `${cars[o.winner].name} wins!`,
        };
      },
      hash: () => race.stateHash(),
      free: () => race.free(),
    };
  },
};
