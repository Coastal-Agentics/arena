// Tank Arena, as an app game module (docs/design/nyborg-app.md §6): its slots, its house
// opponent and how a match link is written. Links are exactly the viewer's tank format:
// ?seed=S&blue=<behavior>-<a>-<s>-<d>&orange=…, then looks and names for the drawing only.
import { encodeBuild, cosmeticsQuery } from "../nyborg-link.js";
import { drawTankScene, TEAM_COLORS, TANK_RADIUS } from "../tank-draw.js";
import { drawYouMarker } from "../app/marker.js";
import { TRI, pointToWeights, weightsToPoint, loadoutWeights, readout, presetName } from "../tank-ui.js";
import { el } from "../app/ui.js";
import { verdict, statusLine } from "../app/build-editor.js";

/** "4-3-2" <-> {attack: 4, speed: 3, defense: 2} (the tank loadout order is A/S/D). */
export const loadoutOf = (levels) => `${levels.attack}-${levels.speed}-${levels.defense}`;
export const levelsOf = (loadout) => { const [attack, speed, defense] = loadout.split("-").map(Number); return { attack, speed, defense }; };

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

  /**
   * The tank build control is the loadout triangle (decision 8 on #85): drag or tap anywhere
   * and the engine snaps to one of the 19 nine-point loadouts; arrow keys move one point.
   * Returns a self-updating element; onChange(build) on every edit.
   */
  editor(engine, start, onChange) {
    const tcat = engine.tankCatalog();
    let b = start;
    const root = el("div", { class: "build-editor tank-editor" });
    const pretty = (l) => l.replaceAll("-", "/");
    const tri = el("div", { class: "tri-wrap" });
    tri.innerHTML = `<svg class="tri" viewBox="0 0 220 214" role="slider" aria-label="Tank loadout triangle: attack, speed, defense" tabindex="0">
      <polygon points="${[TRI.A, TRI.S, TRI.D].map((p) => p.join(",")).join(" ")}" fill="#E6EDF5" stroke="#8FA6C0" stroke-width="1.5" />
      ${tcat.loadouts.map((l) => { const [x, y] = weightsToPoint(loadoutWeights(l)); return `<circle cx="${x}" cy="${y}" r="3" fill="#8FA6C0" />`; }).join("")}
      <circle class="dot" r="8" fill="#436A95" stroke="#fff" stroke-width="2.5" />
      <text x="110" y="9" text-anchor="middle">Attack</text><text x="4" y="206">Speed</text><text x="216" y="206" text-anchor="end">Defense</text>
    </svg>`;
    const svg = tri.querySelector("svg"), dot = tri.querySelector(".dot");
    const presets = el("div", { class: "presets" });
    const read = el("dl", { class: "readout" });
    const beh = el("select", { class: "beh", "aria-label": "Behavior", onchange: (e) => set({ ...b, behavior: { kind: "scripted", id: e.target.value } }) },
      tcat.behaviors.map(([k, name]) => el("option", { value: k, text: `${name} · scripted` })));
    const status = el("div");
    const set = (next) => { if (JSON.stringify(next) === JSON.stringify(b)) return; b = next; onChange(b); paint(); };
    const setLoadout = (l) => set({ ...b, levels: levelsOf(l) });
    const paint = () => {
      const l = loadoutOf(b.levels);
      const known = tcat.loadouts.includes(l);
      const [x, y] = weightsToPoint(loadoutWeights(known ? l : "3-3-3"));
      dot.setAttribute("cx", x); dot.setAttribute("cy", y);
      svg.setAttribute("aria-valuetext", pretty(l));
      beh.value = b.behavior?.id || "";
      presets.replaceChildren(...tcat.presets.map(([name, pl]) => el("button", { type: "button", class: `btn small${pl === l ? " on" : ""}`, text: `${name} ${pretty(pl)}`, onclick: () => setLoadout(pl) })));
      if (known) {
        const r = readout(tcat, l, "3-3-3");
        const preset = presetName(tcat, l);
        read.innerHTML = `<dt>Build</dt><dd>${pretty(l)} (A/S/D)${preset ? ` · ${preset}` : ""}</dd><dt>Damage</dt><dd>${r.damage} per hit</dd>
          <dt>Reload</dt><dd>${r.reloadSec} s</dd><dt>Speed</dt><dd>${r.speed} u/s · turns ${r.turnDegPerSec}°/s</dd><dt>HP</dt><dd>${r.hp}</dd>
          <dt>Vs Balanced</dt><dd>${r.hitsToKill} hits to win</dd>`;
      } else read.textContent = "";
      status.replaceChildren(statusLine(verdict(engine, "tank", b)));
    };
    const pick = (ev) => {
      const pt = svg.createSVGPoint();
      pt.x = ev.clientX; pt.y = ev.clientY;
      const p = pt.matrixTransform(svg.getScreenCTM().inverse());
      const [a, s, d] = pointToWeights(p.x, p.y);
      setLoadout(engine.wasm.snapLoadout(a, s, d)); // the Rust snap: always one of the 19
    };
    svg.addEventListener("pointerdown", (ev) => { svg.setPointerCapture(ev.pointerId); pick(ev); });
    svg.addEventListener("pointermove", (ev) => { if (svg.hasPointerCapture(ev.pointerId)) pick(ev); });
    svg.addEventListener("keydown", (ev) => {
      const w = loadoutWeights(loadoutOf(b.levels)), st = 1 / 6;
      const m = { ArrowUp: [st, -st / 2, -st / 2], ArrowLeft: [-st / 2, st, -st / 2], ArrowRight: [-st / 2, -st / 2, st], ArrowDown: [-st, st / 2, st / 2] }[ev.key];
      if (!m) return;
      ev.preventDefault();
      setLoadout(engine.wasm.snapLoadout(w[0] + m[0], w[1] + m[1], w[2] + m[2]));
    });
    root.append(
      el("div", { class: "points", text: "9 / 9 points · every spot on the triangle spends the same budget" }),
      el("div", { class: "tri-row" }, tri, read),
      el("h3", { text: "Presets" }), presets,
      el("h3", { text: "Behavior" }), beh, status);
    paint();
    return root;
  },
};
