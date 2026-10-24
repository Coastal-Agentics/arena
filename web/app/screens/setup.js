// Match setup (#/play/<game>?slots=<id>,<id>…&seed=N): who plays, in which slot, on which seed.
// The default is your Nyborg against the house Nyborgs; any slot can swap to one of yours or
// another house Nyborg, and racing grids take 2-4 cars. The hash keeps the setup, so reload and
// Back land where you were. Start opens the match.
import { el, face, titleCase } from "../ui.js?v=20261024b";
import { buildFor, buildStatus } from "../../nyborg-library.js?v=20261024b";
import { summaryLine } from "../../build-tabs.js?v=20261024b";
import { arenas, matchFor } from "../../games/index.js?v=20261024b";
import { HOUSE, isHouse } from "../house.js?v=20261024b";
import { href } from "../router.js?v=20261024b";
import { resolveMatch, seedOk, freshSeed, matchQuery } from "../match.js?v=20261024b";

/** Button label for Start — never the arena `start(engine, match)` method. */
function startCtaLabel(mod) {
  if (typeof mod.startLabel === "string" && mod.startLabel.trim()) return mod.startLabel;
  if (typeof mod.button === "string" && mod.button.trim()) return mod.button;
  return "Start ▶";
}

export default {
  title: (args, app) => `${arenas(app.engine).find((m) => m.id === args.game)?.title || "Play"} setup`,
  mount(root, { args, params, app }) {
    const { engine, lib } = app;
    const mod = arenas(engine).find((m) => m.id === args.game);
    if (!mod) { app.go(href([]), { replace: true }); return; }
    const resolved = resolveMatch(app, mod, params);
    const who = resolved.who;
    let ids = resolved.ids;
    let seed = resolved.seed;
    let seedError = "";

    const save = () => history.replaceState(null, "", href(["play", mod.id], matchQuery(ids, seed)));
    const unusedHouse = () => HOUSE.find((n) => !ids.includes(n.id))?.id || HOUSE[0].id;

    const swapper = (i) => {
      const sel = el("select", { "aria-label": `Who plays ${mod.slots.names[i]}`, onchange: (e) => { ids[i] = e.target.value; render(); } },
        el("optgroup", { label: "Your Nyborgs" }, lib.list.map((n) => el("option", { value: n.id, text: n.name }))),
        el("optgroup", { label: "House Nyborgs" }, HOUSE.map((n) => el("option", { value: n.id, text: n.name }))));
      sel.value = ids[i];
      return sel;
    };

    const slotCard = (id, i) => {
      const n = who(id);
      const st = buildStatus(engine, n, mod.id);
      const line = st.status === "ok" ? summaryLine(engine.catalog(mod.id), buildFor(engine, n, mod.id)) : "plays the default build (rebuild needed)";
      return el("li", { class: `slot${isHouse(id) ? " house" : " mine"}`, "data-id": id },
        el("span", { class: "slot-name", text: mod.slots.names[i] }),
        face(() => who(ids[i]), { scale: 1.05, y: 0.7 }, { class: "slot-face" }),
        el("div", { class: "slot-about" },
          el("div", { class: "row gap" }, el("strong", { class: "nm", text: n.name }), el("span", { class: `tag ${isHouse(id) ? "house" : "yours"}`, text: isHouse(id) ? "House" : "Yours" })),
          el("div", { class: "muted small", text: `${titleCase(mod.title)}: ${line}` }),
          n.tagline ? el("div", { class: "small tagline", text: n.tagline }) : null),
        el("div", { class: "slot-acts" },
          el("label", { class: "small muted" }, "Swap ", swapper(i)),
          ids.length > mod.slots.min ? el("button", { type: "button", class: "btn small", text: "Remove", onclick: () => { ids.splice(i, 1); render(); } }) : null));
    };

    const start = () => {
      const m = matchFor(engine, mod, ids.map(who), seed);
      return app.matchHref(mod, m, ids);
    };

    const render = () => {
      save();
      const seedIn = el("input", { id: "seed", type: "text", inputmode: "numeric", value: seed, "aria-describedby": "seed-help", class: seedError ? "bad" : null,
        onchange: (e) => {
          const v = e.target.value.trim();
          if (seedOk(v)) { seed = v; seedError = ""; } else seedError = "A seed is a whole number from 0 to 18446744073709551615.";
          render();
        } });
      root.replaceChildren(...[
        el("div", { class: "intro" },
          el("p", { class: "eyebrow" }, el("a", { href: href([]), text: "← Your Nyborgs" })),
          el("h1", { tabindex: "-1", text: `${mod.title}: ${mod.setup.toLowerCase()}` }),
          el("p", { class: "muted", text: mod.blurb })),
        el("ol", { class: "slots" }, ids.map(slotCard)),
        ids.length < mod.slots.max ? el("button", { type: "button", class: "btn add-slot", text: "+ Add a house Nyborg", onclick: () => { ids.push(unusedHouse()); render(); } }) : null,
        el("div", { class: "seed-row row gap" },
          el("label", { for: "seed", class: "h3", text: "Seed" }), seedIn,
          el("button", { type: "button", class: "btn small", text: "🎲 Shuffle", onclick: () => { seed = freshSeed(); seedError = ""; render(); } }),
          el("span", { id: "seed-help", class: seedError ? "error small" : "muted small", text: seedError || "Same seed and builds, same match, every time." })),
        el("div", { class: "row gap start-row" }, el("a", { class: "btn primary big", id: "start", href: start(), text: startCtaLabel(mod) })),
      ].filter(Boolean));
    };
    render();
  },
};
