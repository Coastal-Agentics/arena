// Backup (#/backup): export your Nyborgs (all, or one) as .json files and import them back,
// plus any saved entries the browser couldn't read. Formats: docs/design/nyborg-library.md.
import { el, face } from "../ui.js?v=20261024a2";
import { exportAll, exportOne, fileNameFor, parseImport, sanitizeImported, newId } from "../../nyborg-library.js?v=20261024a2";
import { href } from "../router.js?v=20261024a2";

function download(name, text) {
  const a = el("a", { href: URL.createObjectURL(new Blob([text], { type: "application/json" })), download: name });
  document.body.append(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 0);
}

/**
 * Import a file's text into the library. Same id already here: `clash(n)` returns "skip",
 * "both" (save under a new id) or "replace". Returns {added, notes, error}.
 */
export async function importInto(engine, lib, text, clash, safe = (f) => f()) {
  const parsed = parseImport(text);
  if (parsed.error) return { added: 0, notes: [], error: parsed.error };
  let added = 0;
  const notes = [];
  for (const raw of parsed.items) {
    const r = sanitizeImported(engine, raw);
    if (r.error) { notes.push(r.error); continue; }
    let n = r.nyborg;
    if (lib.get(n.id)) {
      const choice = await clash(n);
      if (choice === "skip") continue;
      if (choice === "both") n = { ...n, id: newId() };
    }
    if (safe(() => lib.save(n))) { added++; safe(() => lib.select(n.id)); }
    for (const note of r.notes) notes.push(`${n.name}: ${note}`);
  }
  return { added, notes, error: null };
}

export default {
  title: () => "Backup",
  mount(root, { app }) {
    const { engine, lib } = app;
    const msg = el("div", { "aria-live": "polite" });
    const file = el("input", { type: "file", accept: ".json,application/json", hidden: true, id: "import-file" });
    const ask = (n) => new Promise((resolve) => {
      msg.replaceChildren(el("div", { class: "banner" }, el("span", { text: `${lib.get(n.id).name} is already here.` }),
        el("span", { class: "row gap" }, ["skip", "both", "replace"].map((v) => el("button", { type: "button", class: `btn small${v === "replace" ? " primary" : ""}`,
          text: { skip: "Skip", both: "Keep both", replace: "Replace" }[v], onclick: () => { msg.replaceChildren(); resolve(v); } })))));
    });
    file.addEventListener("change", async (e) => {
      const f = e.target.files?.[0];
      e.target.value = "";
      if (!f) return;
      if (f.size > 1024 * 1024) { app.banner("Couldn't import: file is larger than 1 MB.", "bad"); return; }
      const r = await importInto(engine, lib, await f.text(), ask, app.safe);
      if (r.error) app.banner(`Couldn't import: ${r.error}.`, "bad");
      else app.banner(`Imported ${r.added} Nyborg${r.added === 1 ? "" : "s"}.${r.notes.length ? ` ${r.notes.join("; ")}.` : ""}`, r.notes.length ? "" : "ok");
      render();
    });
    const render = () => root.replaceChildren(
      el("div", { class: "intro" },
        el("p", { class: "eyebrow" }, el("a", { href: href([]), text: "← Your Nyborgs" })),
        el("h1", { tabindex: "-1", text: "Backup" }),
        el("p", { class: "muted", text: "Your Nyborgs live in this browser. Download them as a file to keep them safe or move them to another browser." })),
      msg,
      el("section", { class: "panel backup" },
        el("div", { class: "row gap" },
          el("button", { type: "button", class: "btn primary", id: "export-all", text: `Download all ${lib.list.length} Nyborgs`, disabled: !lib.list.length,
            onclick: () => download(`nyborgs-${new Date().toISOString().slice(0, 10)}.json`, exportAll(lib.list)) }),
          el("button", { type: "button", class: "btn", id: "import", text: "Import a file…", onclick: () => file.click() }), file),
        el("ul", { class: "backup-list" }, lib.list.map((n) => el("li", {},
          face(n, { scale: 0.6, y: 0.7 }, { class: "mini-face" }), el("span", { class: "nm", text: n.name }),
          el("button", { type: "button", class: "btn small", text: "Download", onclick: () => download(fileNameFor(n), exportOne(n)) })))),
        lib.broken.length ? el("div", { class: "broken" },
          el("h3", { text: `${lib.broken.length} saved Nyborg${lib.broken.length === 1 ? "" : "s"} couldn't be read` }),
          lib.broken.map((b) => el("div", { class: "row gap" },
            el("code", { text: b.id }),
            el("button", { type: "button", class: "btn small", text: "Download raw", onclick: () => download(`unreadable-${b.id}.json`, b.raw ?? "") }),
            el("button", { type: "button", class: "btn small danger", text: "Remove", onclick: () => { app.safe(() => lib.remove(b.id)); render(); } })))) : null));
    render();
  },
};
