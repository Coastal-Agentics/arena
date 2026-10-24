// Hatch: make a new Nyborg in one screen (name, hair style, hair colour), then home.
import { el, face } from "../ui.js?v=20261024a2";
import { HAIR_PALETTE, HAIR_STYLES } from "../../nyborg-draw.js?v=20261024a2";
import { DEFAULT_LOOK, cleanName, createNyborg, nextFreeName } from "../../nyborg-library.js?v=20261024a2";
import { href } from "../router.js?v=20261024a2";

export default {
  title: () => "Hatch a Nyborg",
  mount(root, { app }) {
    const { lib, engine } = app;
    const first = lib.list.length === 0;
    const draft = { id: `hatch-${lib.list.length}`, name: nextFreeName(lib.list), look: { ...DEFAULT_LOOK, accessories: [] } };

    const nameIn = el("input", { id: "hatch-name", type: "text", maxlength: "24", value: draft.name, autocomplete: "off", "aria-describedby": "hatch-err" });
    const err = el("p", { id: "hatch-err", class: "error small", hidden: true });
    const styles = el("div", { class: "styles", role: "radiogroup", "aria-label": "Hair style" });
    const swatches = el("div", { class: "swatches", role: "radiogroup", "aria-label": "Hair colour" });

    const paintChoices = () => {
      styles.replaceChildren(...HAIR_STYLES.map((s) => el("button", {
        type: "button", class: "style-btn", role: "radio", title: s.name, "aria-checked": String(draft.look.hair_style === s.id),
        onclick: () => { draft.look.hair_style = s.id; paintChoices(); },
      }, face(() => ({ id: draft.id, look: { ...draft.look, hair_style: s.id } }), { scale: 0.62, y: 0.72 }), el("span", { text: s.name }))));
      swatches.replaceChildren(...HAIR_PALETTE.map((c) => el("button", {
        type: "button", class: "swatch", role: "radio", style: `background:${c.hex}`, title: c.name, "aria-label": c.name,
        "aria-checked": String(draft.look.hair_color === c.hex), onclick: () => { draft.look.hair_color = c.hex; paintChoices(); },
      })));
    };

    const hatch = (e) => {
      e.preventDefault();
      const name = cleanName(nameIn.value);
      if (!name) { err.textContent = "A name needs 1 to 24 characters."; err.hidden = false; nameIn.focus(); return; }
      const n = { ...createNyborg(engine, name), look: { ...draft.look } };
      if (app.safe(() => lib.save(n))) {
        app.safe(() => lib.select(n.id));
        app.go(href([]));
      }
    };

    paintChoices();
    root.replaceChildren(
      el("div", { class: "intro" },
        el("h1", { tabindex: "-1", text: first ? "Say hi! Hatch your first Nyborg" : "Hatch a new Nyborg" }),
        el("p", { class: "muted", text: "Pick a name, a hairdo and a colour. You can change all of it later, and builds start balanced." })),
      el("form", { class: "hatch", onsubmit: hatch },
        el("div", { class: "stage" }, face(() => ({ ...draft, name: nameIn.value }), { fit: 105, flip: true, y: 0.64 }, { class: "big-face" })),
        el("div", { class: "choices" },
          el("label", { for: "hatch-name", class: "h3", text: "Name" }), nameIn, err,
          el("h3", { text: "Hair style" }), styles,
          el("h3", { text: "Hair colour" }), swatches,
          el("div", { class: "row gap actions" },
            el("button", { type: "submit", class: "btn primary big", text: "Hatch! 🥚" }),
            first ? null : el("a", { class: "btn", href: href([]), text: "Cancel" })))));
    nameIn.addEventListener("input", () => { err.hidden = true; });
  },
};
