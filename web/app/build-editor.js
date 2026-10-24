// The schema-driven build editor: one stepper and pip row per stat from the engine's catalog,
// presets and behavior, and the engine's own validator for the verdict (the Customizer's build
// tab, moved here). Games without their own editor in web/games/ get this one.
import { el } from "./ui.js?v=20261024b";
import { tabModel, setLevel, applyPreset, setBehavior, friendlyErrors } from "../build-tabs.js?v=20261024b";

/** Build status for `build`: {ok, text, errors}. */
export function verdict(engine, game, build) {
  const cat = engine.catalog(game);
  const result = engine.validateBuild(game, build);
  return { ok: result.ok, rebuild: !result.ok && result.errors.some((e) => e.code === "rules_version_mismatch"), errors: result.ok ? [] : friendlyErrors(cat, result, build.levels) };
}

/** A self-updating editor element. onChange(build) on every edit (valid or not). */
export function pipsEditor(engine, game, start, onChange) {
  const cat = engine.catalog(game);
  const root = el("div", { class: "build-editor" });
  let b = start;
  const edit = (next) => { b = next; onChange(b); render(); };
  const render = () => {
    const m = tabModel(cat, b);
    const v = verdict(engine, game, b);
    const pointsClass = m.spent === m.budget ? "" : m.spent > m.budget ? "over" : "under";
    const pointsText = m.remaining === 0 ? `${m.spent} / ${m.budget} points · equal budget`
      : m.remaining > 0 ? `${m.spent} / ${m.budget} points · ${m.remaining} left` : `${m.spent} / ${m.budget} points · ${-m.remaining} over`;
    const beh = el("select", { class: "beh", "aria-label": "Behavior", onchange: (e) => edit(setBehavior(b, e.target.value)) },
      m.behavior ? null : el("option", { value: "", text: "Choose…" }),
      m.behaviors.map((x) => el("option", { value: x.id, text: `${x.label} · scripted` })));
    beh.value = m.behavior || "";
    root.replaceChildren(...[
      el("div", { class: `points ${pointsClass}`, text: pointsText }),
      ...m.stats.map((s) => el("div", { class: "stat", "data-stat": s.key },
        el("div", { class: "ctl" }, el("span", { class: "lbl", text: s.label }), s.cost !== 1 ? el("span", { class: "cost", text: `${s.cost} pts/level` }) : null),
        el("div", { class: "lvl", text: s.level ?? "?" }),
        el("div", { class: "ctl" },
          el("button", { type: "button", class: "btn stepper", text: "−", "aria-label": `Lower ${s.label}`, disabled: !s.canDown, onclick: () => edit(setLevel(cat, b, s.key, s.level - 1)) }),
          el("div", { class: "pips", role: "group", "aria-label": `${s.label} level ${s.level} of ${s.max}` },
            Array.from({ length: s.max }, (_, i) => el("button", {
              type: "button", class: `pip${i < (s.level || 0) ? " on" : ""}`, "aria-label": `${s.label} ${i + 1}`,
              disabled: i + 1 < s.min, onclick: () => edit(setLevel(cat, b, s.key, i + 1)),
            }))),
          el("button", { type: "button", class: "btn stepper", text: "+", "aria-label": `Raise ${s.label}`, disabled: !s.canUp, onclick: () => edit(setLevel(cat, b, s.key, s.level + 1)) })),
        el("div", { class: "ro", text: s.readout }))),
      el("h3", { text: "Presets" }),
      el("div", { class: "presets" }, m.presets.map((p) => el("button", {
        type: "button", class: `btn small${p.on ? " on" : ""}`, text: `${p.label} ${cat.stats.map((s) => p.levels[s.key]).join("-")}`, onclick: () => edit(applyPreset(b, p)),
      }))),
      el("h3", { text: "Behavior" }), beh,
      v.rebuild ? el("div", {}, el("button", { type: "button", class: "btn", text: "Rebuild with defaults", onclick: () => edit(engine.defaultBuild(game)) })) : null,
      statusLine(v),
    ].filter(Boolean));
  };
  render();
  return root;
}

export function statusLine(v) {
  return v.ok
    ? el("p", { class: "status ok", text: "✓ Saved. Checked by the same validator the engine uses at match start." })
    : el("div", { class: "status bad" }, el("strong", { text: "Not saved yet. Fix this first:" }), el("ul", { class: "errors" }, v.errors.map((t) => el("li", { text: t }))));
}
