// Home: My Nyborgs (docs/design/nyborg-app.md §3). Your selected Nyborg in the spotlight with a
// button per arena, then every Nyborg as a card. A first visit goes to Hatch instead.
import { el, face, titleCase } from "../ui.js";
import { buildFor, buildStatus } from "../../nyborg-library.js";
import { summaryLine } from "../../build-tabs.js";
import { arenas, quickMatch, viewerLink } from "../../games/index.js";
import { href } from "../router.js";

export default {
  title: () => "Your Nyborgs",
  mount(root, { app }) {
    const { lib, engine } = app;
    if (!lib.list.length && !lib.broken.length) { app.go(href(["new"]), { replace: true }); return; }
    const games = arenas(engine);
    const me = () => lib.get(lib.selected) || lib.list[0];

    const playButtons = (n, cls = "") => games.map((g) => el("a", {
      class: `btn play ${cls}`, href: viewerLink(engine, g, quickMatch(engine, g, n)),
      "data-game": g.id, text: g.button, title: `${g.title}: ${n.name} against the house`,
    }));
    const buildLines = (n) => games.map((g) => {
      const st = buildStatus(engine, n, g.id);
      const text = st.status === "ok" ? summaryLine(engine.catalog(g.id), buildFor(engine, n, g.id)) : st.status === "rebuild" ? "rebuild needed" : "needs fixing";
      return el("div", { class: `line${st.status === "ok" ? "" : " rebuild"}` }, el("strong", { text: `${titleCase(g.title)}: ` }), text);
    });

    const render = () => {
      const n = me();
      const spotlight = el("section", { class: "spotlight", "aria-labelledby": "me-name" },
        el("div", { class: "stage" }, face(() => me(), { fit: 105, flip: true, y: 0.64 }, { class: "big-face" })),
        el("div", { class: "about" },
          el("p", { class: "eyebrow", text: "Your Nyborg" }),
          el("h2", { id: "me-name", class: "me-name", text: n.name }),
          el("div", { class: "lines" }, buildLines(n)),
          el("div", { class: "row gap actions" },
            playButtons(n, "primary"),
            el("a", { class: "btn", href: "../customizer.html", text: "Edit look & builds" })),
          el("p", { class: "muted small", text: "Your Nyborg plays the house Nyborgs. Same build, same link, same match, every time." })));

      const cards = lib.list.map((x) => el("li", { class: `ny-card${x.id === n.id ? " on" : ""}`, "data-id": x.id },
        el("button", {
          class: "pick", type: "button", "aria-pressed": String(x.id === n.id), title: `Make ${x.name} your Nyborg`,
          onclick: () => { app.safe(() => lib.select(x.id)); render(); },
        }, face(x, { scale: 0.95, y: 0.7 }, { class: "card-face" }), el("span", { class: "nm", text: x.name })),
        el("div", { class: "row mini" }, playButtons(x, "small"))));
      cards.push(el("li", { class: "ny-card new" }, el("a", { class: "pick", href: href(["new"]) },
        el("span", { class: "plus", text: "+" }), el("span", { class: "nm", text: "New Nyborg" }))));

      root.replaceChildren(
        el("div", { class: "intro" },
          el("h1", { tabindex: "-1", text: "Your Nyborgs" }),
          el("p", { class: "muted", text: "Pick a Nyborg, send it into an arena, then come back and tweak it." })),
        spotlight,
        el("section", { class: "all", "aria-labelledby": "all-h" },
          el("h2", { id: "all-h", text: `All your Nyborgs · ${lib.list.length}` }),
          el("ul", { class: "ny-grid" }, cards),
          lib.broken.length ? el("p", { class: "muted small", text: `${lib.broken.length} saved Nyborg(s) couldn't be read; open the Customizer to download or remove them.` }) : null),
        el("p", { class: "muted small foot", },
          "Saved in this browser. Export, import and the full editor live in the ", el("a", { href: "../customizer.html", text: "Customizer" }), "."));
    };
    render();
  },
};
