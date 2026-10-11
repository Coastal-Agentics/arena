// Tank Arena, as an app game module (docs/design/nyborg-app.md §6): its slots, its house
// opponent and how a match link is written. Links are exactly the viewer's tank format:
// ?seed=S&blue=<behavior>-<a>-<s>-<d>&orange=…, then looks and names for the drawing only.
import { encodeBuild, cosmeticsQuery } from "../nyborg-link.js";
import { drawTankScene, TEAM_COLORS, TANK_RADIUS } from "../tank-draw.js";
import { drawYouMarker } from "../app/marker.js";

const TICK_HZ = 60;
const WHY = { last_standing: "last tank standing", all_destroyed: "both destroyed", tick_limit: "time ran out" };
const plainQuery = (engine, { seed, slots }) => {
  const part = (s) => encodeBuild(engine.catalog("tank"), s.build);
  return engine.canonicalTankQuery(`seed=${seed}&blue=${part(slots[0])}&orange=${part(slots[1])}`);
};

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
    const q = plainQuery(engine, { seed, slots });
    const cos = cosmeticsQuery(slots.map((s) => ({ look: s.look, name: s.name })));
    return `${q}${cos ? `&${cos}` : ""}`;
  },

  /** Run a match: only the canonical tank query reaches the engine (WasmMatch.tank, as the viewer). */
  start(engine, match) {
    const m = engine.wasm.WasmMatch.tank(plainQuery(engine, match));
    const looks = match.slots.map((s) => s.look || null);
    const seeds = match.slots.map((s, i) => s.id || s.name || `tank-${i}`);
    const names = match.slots.map((s, i) => s.name || this.slots.names[i]);
    const facingById = new Map();
    let state = JSON.parse(m.stateJson());
    return {
      width: state.width, height: state.height, tickHz: TICK_HZ,
      get state() { return state; },
      step(n) { m.step(n); state = JSON.parse(m.stateJson()); },
      over: () => !!state.outcome,
      draw(ctx) {
        drawTankScene(ctx, state, { looks, seeds, facingById });
        for (const t of state.tanks) if (match.you?.includes(t.team)) drawYouMarker(ctx, t.x, state.height - t.y - TANK_RADIUS - 16, { alpha: t.alive ? 1 : 0.45 });
      },
      clock: () => `${(state.tick / TICK_HZ).toFixed(1)}s / ${(state.max_ticks / TICK_HZ).toFixed(0)}s`,
      board: () => state.tanks.map((t) => ({
        slot: t.team, name: names[t.team], hair: looks[t.team]?.hair_color || TEAM_COLORS[t.team],
        place: null, note: t.alive ? `${Math.max(0, Math.round(t.hp))} / ${t.max_hp} HP` : "knocked out",
        hp: Math.max(0, t.hp / t.max_hp),
      })),
      result() {
        const o = state.outcome;
        if (!o) return null;
        const order = o.winner == null ? [0, 1] : [o.winner, 1 - o.winner];
        return {
          winner: o.winner ?? null, order, seconds: o.ticks / TICK_HZ,
          place: (slot) => (o.winner == null ? null : slot === o.winner ? 1 : 2),
          time: () => o.ticks / TICK_HZ,
          detail: (slot) => (o.winner == null ? WHY[o.reason] || o.reason : slot === o.winner ? WHY[o.reason] || o.reason : "knocked out"),
          headline: o.winner == null ? "It's a draw!" : `${names[o.winner]} wins!`,
        };
      },
      hash: () => m.stateHash(),
      free: () => m.free(),
    };
  },
};
