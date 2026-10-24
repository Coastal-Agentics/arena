// Edit (#/nyborg/<id>?arena=<game>): your Nyborg's look and its build for every arena, saved as
// you go (only builds the engine accepts are stored). Games bring their own build control
// through web/games/ (the tank triangle), else the schema-driven pips. Then: try it, duplicate,
// delete.
import { el, face, fill } from "../ui.js?v=20261024a2";
import { HAIR_PALETTE, HAIR_STYLES, ACCESSORIES } from "../../nyborg-draw.js?v=20261024a2";
import { buildFor, cleanName, duplicateNyborg, touch } from "../../nyborg-library.js?v=20261024a2";
import { arenas } from "../../games/index.js?v=20261024a2";
import { pipsEditor } from "../build-editor.js?v=20261024a2";
import { href } from "../router.js?v=20261024a2";

export default {
  title: (args, app) => `Edit ${app.lib.get(args.id)?.name || "Nyborg"}`,
  mount(root, { args, params, app }) {
    const { engine, lib } = app;
    const id = args.id;
    if (!lib.get(id)) { app.banner("That Nyborg isn't in this browser.", "", "missing"); app.go(href([]), { replace: true }); return; }
    if (lib.selected !== id) app.safe(() => lib.select(id));
    if (lib.readOnly.has(id)) app.banner("This Nyborg was saved by a newer version of the app, so it's shown but not changed here.", "", "readonly");
    const games = arenas(engine);
    let tab = games.some((g) => g.id === params.get("arena")) ? params.get("arena") : games[0].id;
    const cur = () => lib.get(id);
    const save = (n) => app.safe(() => lib.save(touch(n)));

    // --- look ---
    const lookBox = el("div", { class: "look-controls" });
    const nameIn = el("input", { id: "edit-name", type: "text", maxlength: "24", value: cur().name, autocomplete: "off" });
    const nameErr = el("p", { class: "error small", hidden: true });
    const commitName = () => {
      const name = cleanName(nameIn.value);
      if (!name) { nameErr.textContent = "A name needs 1 to 24 characters."; nameErr.hidden = false; return; }
      nameErr.hidden = true;
      nameIn.value = name;
      if (name !== cur().name && save({ ...cur(), name })) { heading.textContent = name; renderTry(); }
    };
    nameIn.addEventListener("change", commitName);
    nameIn.addEventListener("keydown", (e) => { if (e.key === "Enter") { e.preventDefault(); nameIn.blur(); } });
    const updateLook = (change) => { if (save({ ...cur(), look: { ...cur().look, ...change } })) renderLook(); };
    const renderLook = () => {
      const n = cur();
      fill(lookBox,
        el("h3", { text: "Hair style" }),
        el("div", { class: "styles", role: "radiogroup", "aria-label": "Hair style" }, HAIR_STYLES.map((s) => el("button", {
          type: "button", class: "style-btn", role: "radio", title: s.name, "aria-checked": String(n.look.hair_style === s.id), onclick: () => updateLook({ hair_style: s.id }),
        }, face(() => cur() && { id, look: { ...cur().look, hair_style: s.id, accessories: [] } }, { scale: 0.62, y: 0.72 }), el("span", { text: s.name })))),
        el("h3", { text: "Hair colour" }),
        el("div", { class: "swatches", role: "radiogroup", "aria-label": "Hair colour" }, HAIR_PALETTE.map((c) => el("button", {
          type: "button", class: "swatch", role: "radio", style: `background:${c.hex}`, title: c.name, "aria-label": c.name,
          "aria-checked": String(n.look.hair_color === c.hex), onclick: () => updateLook({ hair_color: c.hex }),
        }))),
        n.look.hair_style === "yarn" ? el("h3", { text: "Strands" }) : null,
        n.look.hair_style === "yarn" ? el("div", { class: "seg", role: "radiogroup", "aria-label": "Strands" }, [2, 3, 4].map((k) => el("button", {
          type: "button", role: "radio", "aria-checked": String(n.look.strands === k), text: String(k), onclick: () => updateLook({ strands: k }),
        }))) : null,
        el("h3", { text: "Accessories" }),
        el("div", { class: "seg" }, ACCESSORIES.map((a) => {
          const on = n.look.accessories.includes(a.id);
          return el("button", { type: "button", "aria-pressed": String(on), text: `${on ? "✓ " : ""}${a.name}`,
            onclick: () => updateLook({ accessories: on ? n.look.accessories.filter((x) => x !== a.id) : [...n.look.accessories, a.id] }) });
        })),
        el("p", { class: "muted small", text: "The look is saved with your Nyborg but never sent to the engine. Two Nyborgs with the same build play exactly the same match." }));
    };

    // --- builds ---
    const drafts = {};
    const tabs = el("div", { class: "tabs", role: "tablist", "aria-label": "Arenas" });
    const panel = el("div", { class: "build-panel", role: "tabpanel" });
    const flag = (g) => !engine.validateBuild(g.id, drafts[g.id] || buildFor(engine, cur(), g.id)).ok;
    const renderTabs = () => tabs.replaceChildren(...games.map((g) => el("button", {
      type: "button", role: "tab", id: `tab-${g.id}`, "aria-selected": String(g.id === tab),
      onclick: () => { tab = g.id; history.replaceState(null, "", href(["nyborg", id], `arena=${tab}`)); renderTabs(); renderPanel(); },
    }, g.title, flag(g) ? el("span", { class: "flag", text: "●", title: "needs attention" }) : null)));
    const onBuild = (game) => (b) => {
      drafts[game] = b;
      // Only a build the engine accepts is stored, so a saved Nyborg can never carry an advantage.
      if (engine.validateBuild(game, b).ok) save({ ...cur(), builds: { ...cur().builds, [game]: b } });
      renderTabs();
    };
    const renderPanel = () => {
      const mod = games.find((g) => g.id === tab);
      const b = drafts[tab] || buildFor(engine, cur(), tab);
      panel.setAttribute("aria-labelledby", `tab-${tab}`);
      panel.replaceChildren(mod.editor ? mod.editor(engine, b, onBuild(tab)) : pipsEditor(engine, tab, b, onBuild(tab)));
    };

    // --- try it, duplicate, delete ---
    const tryRow = el("div", { class: "row gap try-row" });
    const renderTry = () => tryRow.replaceChildren(
      ...games.map((g) => el("a", { class: "btn primary play", href: href(["play", g.id], `me=${encodeURIComponent(id)}`), "data-game": g.id, text: `${g.button.replace(" ▶", "")} ${cur().name} ▶` })),
      el("button", { type: "button", class: "btn", text: "Duplicate", onclick: () => {
        const copy = duplicateNyborg(cur());
        if (app.safe(() => lib.save(copy, { after: id }))) { app.banner(`Made ${copy.name}.`, "ok"); app.go(href(["nyborg", copy.id])); }
      } }),
      el("button", { type: "button", class: "btn danger", id: "delete", text: "Delete…", onclick: confirmDelete }));
    function confirmDelete() {
      const n = cur();
      tryRow.replaceChildren(el("span", { class: "confirm", text: `Delete ${n.name}? ` }),
        el("button", { type: "button", class: "btn danger", id: "confirm-delete", text: "Yes, delete", onclick: () => {
          const index = lib.list.findIndex((x) => x.id === id);
          app.safe(() => lib.remove(id));
          app.undoable(`Deleted ${n.name}.`, () => { const prev = lib.list[index - 1]?.id; if (app.safe(() => lib.save(n, { after: prev }))) app.safe(() => lib.select(n.id)); });
          app.go(href([]));
        } }),
        el("button", { type: "button", class: "btn", text: "Keep", onclick: renderTry }));
    }

    const heading = el("h1", { tabindex: "-1", text: cur().name });
    root.replaceChildren(
      el("div", { class: "intro" },
        el("p", { class: "eyebrow" }, el("a", { href: href([]), text: "← Your Nyborgs" })),
        heading,
        el("p", { class: "muted", text: "Change the look any time; it never changes how your Nyborg plays. Builds save as soon as the engine accepts them." })),
      el("div", { class: "edit-grid" },
        el("section", { class: "panel look", "aria-label": "Look" },
          el("div", { class: "stage" }, face(() => cur(), { fit: 105, flip: true, y: 0.64 }, { class: "big-face" })),
          el("label", { for: "edit-name", class: "h3", text: "Name" }), nameIn, nameErr, lookBox),
        el("section", { class: "panel builds", "aria-label": "Builds" },
          el("h2", { text: "Builds" }),
          el("p", { class: "muted small", text: "One build per arena. Every Nyborg gets the same point budget." }),
          tabs, panel)),
      tryRow);
    renderLook(); renderTabs(); renderPanel(); renderTry();
  },
};
