// The Nyborg Customizer (first slice): a library of Nyborgs, a look editor with a live
// preview (the same drawing as the arena viewer), one schema-driven build tab per game from
// the engine's games() / catalogJson(game), export and import. Save format and storage:
// docs/design/nyborg-library.md. The look never reaches the engine.
import init, { games, catalogJson, defaultBuild, validateBuild, canonicalTankQuery } from "./pkg/engine_wasm.js";
import { drawNyborg, idleBob, HAIR_PALETTE, ACCESSORIES } from "./nyborg-draw.js";
import {
  Library, MemoryStorage, KEYS, cleanName, createNyborg, duplicateNyborg, nextFreeName, touch,
  buildFor, buildStatus, sanitizeImported, parseImport, exportOne, exportAll, fileNameFor, newId,
} from "./nyborg-library.js";
import { tabModel, setLevel, applyPreset, setBehavior, summaryLine, friendlyErrors } from "./build-tabs.js";
import { encodeBuild, raceQuery, cosmeticsQuery } from "./nyborg-link.js";

const $ = (id) => document.getElementById(id);
const el = (tag, attrs = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") e.className = v;
    else if (k === "text") e.textContent = v;
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) e.setAttribute(k, v === true ? "" : v);
  }
  for (const k of kids) if (k != null) e.append(k);
  return e;
};
const titleCase = (s) => s.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

await init();

// The engine, as plain objects. Every game the engine lists gets a tab; nothing below names one.
const engine = {
  games: () => JSON.parse(games()),
  defaultBuild: (g) => JSON.parse(defaultBuild(g)),
  validateBuild: (g, b) => JSON.parse(validateBuild(g, typeof b === "string" ? b : JSON.stringify(b))),
};
const GAMES = engine.games().map((g) => g.game);
const CATALOGS = Object.fromEntries(GAMES.map((g) => [g, JSON.parse(catalogJson(g))]));

// Storage: localStorage when the browser allows it; otherwise this session only.
let storage;
let storageBlocked = false;
try {
  const probe = "arena.nyborg.__probe";
  localStorage.setItem(probe, "1");
  localStorage.removeItem(probe);
  storage = localStorage;
} catch {
  storage = new MemoryStorage();
  storageBlocked = true;
}

const lib = new Library(storage);
lib.load();
const seeded = safe(() => lib.seedIfEmpty(engine));

let gameTab = GAMES[0];
let drafts = {}; // game -> build on screen for the selected Nyborg (may be invalid)
let draftFor = null;
let undo = null; // {nyborg, index, timer}

// --- persistence ---------------------------------------------------------------------

function safe(fn) {
  try { return fn(); } catch (e) {
    if (e && (e.name === "QuotaExceededError" || e.code === 22)) {
      banner("Couldn't save: browser storage is full. Export your library to keep this change.", "bad", "quota");
    } else banner(`Couldn't save: ${e.message || e}`, "bad");
    return null;
  }
}

function saveNyborg(n, opts) {
  return safe(() => lib.save(touch(n), opts));
}

// --- banners, toast, modal -----------------------------------------------------------

function banner(text, kind = "", id = "") {
  if (id && document.querySelector(`[data-banner="${id}"]`)) return;
  const b = el("div", { class: `banner ${kind}`, "data-banner": id || null },
    el("span", { text }),
    el("button", { class: "btn small", type: "button", text: "Dismiss", onclick: () => b.remove() }));
  $("banners").append(b);
}

function toast(text, action) {
  const t = $("toast");
  t.replaceChildren(el("span", { text }));
  if (action) t.append(el("button", { class: "btn small", type: "button", text: action.label, onclick: action.run }));
  t.hidden = false;
}
const hideToast = () => { $("toast").hidden = true; };

function ask(message, buttons) {
  return new Promise((resolve) => {
    $("modal-msg").textContent = message;
    $("modal-btns").replaceChildren(...buttons.map((b) =>
      el("button", { type: "button", class: `btn ${b.kind || ""}`, text: b.label, onclick: () => { $("modal").hidden = true; resolve(b.value); } })));
    $("modal").hidden = false;
    $("modal-btns").lastChild?.focus();
  });
}

function download(name, text) {
  const a = el("a", { href: URL.createObjectURL(new Blob([text], { type: "application/json" })), download: name });
  document.body.append(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 0);
}

// --- previews (one animation loop for every canvas) ----------------------------------

const previews = new Set(); // {canvas, look(), scale, flip}

function addPreview(canvas, look, { scale, flip = false }) {
  const p = { canvas, look, scale, flip };
  previews.add(p);
  return p;
}

function paint(p, t) {
  const c = p.canvas;
  if (!c.isConnected) { previews.delete(p); return; }
  const dpr = window.devicePixelRatio || 1;
  const w = c.clientWidth || c.width, h = c.clientHeight || c.height;
  if (c.width !== Math.round(w * dpr)) { c.width = Math.round(w * dpr); c.height = Math.round(h * dpr); }
  const ctx = c.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  const look = p.look();
  if (!look) return;
  const tick = (t / 1000) * 60;
  // The big preview turns around every 2.4 s, to show the flip the viewer does.
  const facing = p.flip && Math.floor(t / 2400) % 2 === 1 ? -1 : 1;
  ctx.save();
  ctx.translate(w / 2, h * 0.7);
  drawNyborg(ctx, {
    hair: look.hair_color, strands: look.strands, accessories: look.accessories,
    facing, bob: idleBob(tick) * p.scale, scale: p.scale,
  });
  ctx.restore();
}

function loop(t) {
  for (const p of previews) paint(p, t);
  requestAnimationFrame(loop);
}

// --- library -------------------------------------------------------------------------

const selected = () => lib.get(lib.selected);

function renderLibrary() {
  $("lib-count").textContent = String(lib.list.length);
  $("lib-list").replaceChildren(...lib.list.map((n) => {
    const lines = GAMES.map((g) => {
      const st = buildStatus(engine, n, g);
      const name = titleCase(g);
      if (st.status === "rebuild") return el("div", { class: "line rebuild", text: `${name}: rebuild needed` });
      if (st.status === "invalid") return el("div", { class: "line rebuild", text: `${name}: needs fixing` });
      return el("div", { class: "line", text: `${name}: ${summaryLine(CATALOGS[g], buildFor(engine, n, g))}` });
    });
    const canvas = el("canvas", { width: 64, height: 64, "aria-hidden": "true" });
    const card = el("li", { class: `lib-card${n.id === lib.selected ? " on" : ""}`, "data-id": n.id, onclick: () => selectNyborg(n.id) },
      canvas,
      el("div", {}, el("div", { class: "nm", text: n.name }), ...lines),
      el("div", { class: "acts" },
        el("button", { type: "button", class: "btn small", text: "Edit", onclick: (e) => { e.stopPropagation(); selectNyborg(n.id); $("name").focus(); } }),
        el("button", { type: "button", class: "btn small", text: "Duplicate", onclick: (e) => { e.stopPropagation(); duplicate(n.id); } }),
        el("button", { type: "button", class: "btn small danger", text: "Delete", onclick: (e) => { e.stopPropagation(); remove(n.id); } })));
    addPreview(canvas, () => lib.get(n.id)?.look, { scale: 0.62 });
    return card;
  }));
  renderBroken();
  $("editor").hidden = !selected();
}

function renderBroken() {
  const box = $("lib-broken");
  if (!lib.broken.length) { box.replaceChildren(); return; }
  const k = lib.broken.length;
  box.replaceChildren(el("div", { class: "broken" },
    el("div", { text: `${k} Nyborg${k === 1 ? "" : "s"} couldn't be read.` }),
    ...lib.broken.map((b) => el("div", { class: "row gap" },
      el("button", { type: "button", class: "btn small", text: "Download raw", onclick: () => download(`unreadable-${b.id}.json`, b.raw ?? "") }),
      el("button", { type: "button", class: "btn small danger", text: "Remove", onclick: async () => {
        if (await ask("Remove this unreadable entry? Download it first if you want to keep it.", [{ label: "Cancel", value: false }, { label: "Remove", value: true, kind: "danger" }])) {
          safe(() => lib.remove(b.id)); renderAll();
        }
      } })))));
}

function selectNyborg(id) {
  safe(() => lib.select(id));
  renderAll();
}

function createNew() {
  const n = createNyborg(engine, nextFreeName(lib.list));
  if (saveNyborg(n)) { lib.selected = n.id; safe(() => lib.select(n.id)); renderAll(); }
}

function duplicate(id) {
  const src = lib.get(id);
  if (!src) return;
  const copy = duplicateNyborg(src);
  if (saveNyborg(copy, { after: id })) { safe(() => lib.select(copy.id)); renderAll(); toast(`Made ${copy.name}.`); setTimeout(hideToast, 2500); }
}

async function remove(id) {
  const n = lib.get(id);
  if (!n) return;
  const ok = await ask(`Delete ${n.name}?`, [{ label: "Cancel", value: false }, { label: "Delete", value: true, kind: "danger" }]);
  if (!ok) return;
  const index = lib.list.findIndex((x) => x.id === id);
  safe(() => lib.remove(id));
  if (undo) clearTimeout(undo.timer);
  undo = { nyborg: n, index, timer: setTimeout(() => { undo = null; hideToast(); }, 10000) };
  toast(`Deleted "${n.name}".`, { label: "Undo", run: () => {
    if (!undo) return;
    clearTimeout(undo.timer);
    const prev = lib.list[undo.index - 1]?.id;
    if (safe(() => lib.save(undo.nyborg, { after: prev }))) { safe(() => lib.select(undo.nyborg.id)); }
    undo = null; hideToast(); renderAll();
  } });
  renderAll();
}

// --- editor: name and look -----------------------------------------------------------

function renderEditor() {
  const n = selected();
  if (!n) return;
  if (draftFor !== n.id) { drafts = {}; draftFor = n.id; }
  if (document.activeElement !== $("name")) $("name").value = n.name;
  $("name-error").hidden = true;

  $("swatches").replaceChildren(...HAIR_PALETTE.map((c) => el("button", {
    type: "button", class: "swatch", role: "radio", style: `background:${c.hex}`, title: c.name,
    "aria-label": c.name, "aria-checked": String(n.look.hair_color === c.hex),
    onclick: () => updateLook({ hair_color: c.hex }),
  })));
  $("strands").replaceChildren(...[2, 3, 4].map((k) => el("button", {
    type: "button", role: "radio", "aria-checked": String(n.look.strands === k), text: String(k),
    onclick: () => updateLook({ strands: k }),
  })));
  $("accessories").replaceChildren(...ACCESSORIES.map((a) => {
    const on = n.look.accessories.includes(a.id);
    return el("button", {
      type: "button", "aria-pressed": String(on), text: `${on ? "✓ " : ""}${a.name}`,
      onclick: () => updateLook({ accessories: on ? n.look.accessories.filter((x) => x !== a.id) : [...n.look.accessories, a.id] }),
    });
  }));
  renderGameTabs();
  renderPick();
}

function updateLook(change) {
  const n = selected();
  if (!n) return;
  if (saveNyborg({ ...n, look: { ...n.look, ...change } })) renderAll();
}

function commitName() {
  const n = selected();
  if (!n) return;
  const name = cleanName($("name").value);
  if (!name) { $("name-error").textContent = "A name needs 1 to 24 characters."; $("name-error").hidden = false; return; }
  $("name").value = name;
  if (name !== n.name && saveNyborg({ ...n, name })) renderLibrary();
}

// --- editor: one tab per game, built from the catalog --------------------------------

function draft(game) {
  if (!drafts[game]) drafts[game] = buildFor(engine, selected(), game);
  return drafts[game];
}

function renderGameTabs() {
  const n = selected();
  $("game-tabs").replaceChildren(...GAMES.map((g) => {
    const st = buildStatus(engine, n, g).status;
    const draftOk = !drafts[g] || engine.validateBuild(g, drafts[g]).ok;
    return el("button", {
      type: "button", role: "tab", "aria-selected": String(g === gameTab), id: `tab-${g}`,
      onclick: () => { gameTab = g; renderGameTabs(); },
    }, titleCase(g), st !== "ok" || !draftOk ? el("span", { class: "flag", text: "●", title: "needs attention" }) : null);
  }));
  renderGamePanel(gameTab);
}

function renderGamePanel(game) {
  const cat = CATALOGS[game];
  const b = draft(game);
  const m = tabModel(cat, b);
  const result = engine.validateBuild(game, b);
  const errs = friendlyErrors(cat, result, b.levels);
  const rebuild = !result.ok && result.errors.some((e) => e.code === "rules_version_mismatch");
  const pointsClass = m.spent === m.budget ? "" : m.spent > m.budget ? "over" : "under";
  const pointsText = m.remaining === 0 ? `${m.spent} / ${m.budget} points · equal budget`
    : m.remaining > 0 ? `${m.spent} / ${m.budget} points · ${m.remaining} left`
      : `${m.spent} / ${m.budget} points · ${-m.remaining} over`;

  const panel = $("game-panel");
  panel.setAttribute("aria-labelledby", `tab-${game}`);
  panel.replaceChildren(...[
    el("p", { class: "muted small", text: "Stats, ranges and points come from the engine's catalog for this game. Every Nyborg gets the same budget." }),
    el("div", { class: `points ${pointsClass}`, id: "points", text: pointsText }),
    ...m.stats.map((s) => el("div", { class: "stat", "data-stat": s.key },
      el("div", { class: "ctl" },
        el("span", { class: "lbl", text: s.label }),
        s.cost !== 1 ? el("span", { class: "cost", text: `${s.cost} pts/level` }) : null),
      el("div", { class: "lvl", text: s.level ?? "?" }),
      el("div", { class: "ctl" },
        el("button", { type: "button", class: "btn stepper", text: "−", "aria-label": `Lower ${s.label}`, disabled: !s.canDown, onclick: () => editBuild(game, setLevel(cat, b, s.key, s.level - 1)) }),
        el("div", { class: "pips", role: "group", "aria-label": `${s.label} level ${s.level} of ${s.max}` },
          ...Array.from({ length: s.max }, (_, i) => el("button", {
            type: "button", class: `pip${i < (s.level || 0) ? " on" : ""}`, "aria-label": `${s.label} ${i + 1}`,
            disabled: i + 1 < s.min, onclick: () => editBuild(game, setLevel(cat, b, s.key, i + 1)),
          }))),
        el("button", { type: "button", class: "btn stepper", text: "+", "aria-label": `Raise ${s.label}`, disabled: !s.canUp, onclick: () => editBuild(game, setLevel(cat, b, s.key, s.level + 1)) })),
      el("div", { class: "ro", text: s.readout }))),
    el("h3", { text: "Presets" }),
    el("div", { class: "presets" }, ...m.presets.map((p) => el("button", {
      type: "button", class: `btn small${p.on ? " on" : ""}`, text: `${p.label} ${cat.stats.map((s) => p.levels[s.key]).join("-")}`,
      onclick: () => editBuild(game, applyPreset(b, p)),
    }))),
    el("h3", { text: "Behavior" }),
    (() => {
      const sel = el("select", { class: "beh", "aria-label": "Behavior", onchange: (e) => editBuild(game, setBehavior(b, e.target.value)) },
        ...(m.behavior ? [] : [el("option", { value: "", text: "Choose…" })]),
        ...m.behaviors.map((x) => el("option", { value: x.id, text: `${x.label} · scripted` })));
      sel.value = m.behavior || "";
      return sel;
    })(),
    rebuild ? el("div", {}, el("button", { type: "button", class: "btn", text: "Rebuild with defaults", onclick: () => editBuild(game, engine.defaultBuild(game)) })) : null,
    result.ok
      ? el("p", { class: "status ok", id: "build-status", text: "✓ Saved. Checked by the same validator the engine uses at match start." })
      : el("div", { class: "status bad", id: "build-status" },
        el("strong", { text: "Not saved yet. Fix this first:" }),
        el("ul", { class: "errors" }, ...errs.map((t) => el("li", { text: t })))),
  ].filter(Boolean));
}

function editBuild(game, build) {
  drafts[game] = build;
  const r = engine.validateBuild(game, build);
  // Only a build the engine accepts is stored, so a saved Nyborg can never carry an advantage.
  if (r.ok) {
    const n = selected();
    saveNyborg({ ...n, builds: { ...n.builds, [game]: build } });
    renderLibrary();
  }
  renderGameTabs();
  renderPick();
}

// --- pick for a match ----------------------------------------------------------------
// Links into the viewer, per game. The builds play the match; looks and names ride along for
// the drawing only (nyborg-link.js), so a shared link plays the same match for anyone.
// Tank: ?seed=S&blue=<behavior>-<a>-<s>-<d>&orange=… (wasm-and-web.md), then cosmetics.
// Racing: ?game=racing&seed=S&cars=<behavior>-<p>-<t>-<g>,… (viewer-multi-game.md §4), then cosmetics.
const U64_MAX = 18446744073709551615n;
const seedOk = (s) => /^\d{1,20}$/.test(s) && BigInt(s) <= U64_MAX;
const WATCH_LINKS = {
  tank: {
    button: "Watch ▶",
    min: 2, max: 2,
    label: (i) => ["Blue", "Orange"][i],
    cls: (i) => ["slot-blue", "slot-orange"][i],
    between: "vs",
    ids: (chosen) => [chosen.blue, chosen.orange],
    save: (ids) => ({ blue: ids[0], orange: ids[1] }),
    url: (seed, nyborgs, builds) => {
      const part = (b) => encodeBuild(CATALOGS.tank, b);
      const q = canonicalTankQuery(`seed=${seed}&blue=${part(builds[0])}&orange=${part(builds[1])}`);
      const cos = cosmeticsQuery(nyborgs.map((n) => ({ look: n.look, name: n.name })));
      return `./arena.html?${q}${cos ? `&${cos}` : ""}`;
    },
  },
  racing: {
    button: "Race these ▶",
    min: 2, max: 4,
    label: (i) => `Car ${i + 1}`,
    cls: () => "slot-car",
    between: "",
    ids: (chosen) => (Array.isArray(chosen.cars) ? chosen.cars : []),
    save: (ids) => ({ cars: ids }),
    url: (seed, nyborgs, builds) => {
      if (!seedOk(seed)) throw new Error("The seed must be a whole number from 0 to 18446744073709551615.");
      return `./arena.html?${raceQuery(CATALOGS.racing, {
        seed, builds, custom: true, looks: nyborgs.map((n) => n.look), names: nyborgs.map((n) => n.name),
      })}`;
    },
  },
};

function readPicks() {
  try { return JSON.parse(storage.getItem(KEYS.picks) || "{}") || {}; } catch { return {}; }
}

function savePick(game, value) {
  safe(() => storage.setItem(KEYS.picks, JSON.stringify({ ...readPicks(), [game]: value })));
}

function renderPick() {
  const box = $("pick");
  const picks = readPicks();
  const rows = [];
  for (const [game, spec] of Object.entries(WATCH_LINKS)) {
    if (!GAMES.includes(game)) continue;
    const chosen = picks[game] || {};
    // Slot count: the saved one, else as many Nyborgs as you have (within the game's range).
    const saved = spec.ids(chosen);
    const count = Math.min(spec.max, Math.max(spec.min, saved.length || lib.list.length));
    const sels = Array.from({ length: count }, (_, i) => {
      const fallback = lib.list.length ? lib.list[i % lib.list.length].id : "";
      const id = lib.get(saved[i]) ? saved[i] : fallback;
      const s = el("select", { class: spec.cls(i), "aria-label": `${titleCase(game)} ${spec.label(i)}`, "data-slot": String(i) },
        ...lib.list.map((n) => el("option", { value: n.id, text: n.name })));
      s.value = id || "";
      return s;
    });
    const seed = el("input", { value: chosen.seed || "42", inputmode: "numeric", "aria-label": `${titleCase(game)} seed` });
    const go = el("a", { class: "btn primary", href: "#", text: spec.button, "data-game": game });
    const err = el("span", { class: "error small" });
    const remember = (ids) => savePick(game, { ...spec.save(ids), seed: seed.value.trim() });
    const update = (save) => {
      const ids = sels.map((s) => s.value);
      const nyborgs = ids.map((id) => lib.get(id));
      const builds = nyborgs.map((n) => n && buildFor(engine, n, game));
      const bad = builds.findIndex((b) => !b || !engine.validateBuild(game, b).ok);
      // Saved only when the user picks, so the defaults keep following the library.
      if (save === true) remember(ids);
      try {
        if (bad >= 0) throw new Error(`${nyborgs[bad]?.name || "A Nyborg"} needs a valid ${titleCase(game)} build first.`);
        go.href = spec.url(seed.value.trim() || "0", nyborgs, builds);
        go.removeAttribute("aria-disabled");
        err.textContent = "";
      } catch (e) {
        go.href = "#";
        go.setAttribute("aria-disabled", "true");
        err.textContent = String(e.message || e);
      }
    };
    for (const s of sels) s.addEventListener("change", () => update(true));
    seed.addEventListener("input", () => update(true));
    go.addEventListener("click", (e) => { if (go.getAttribute("aria-disabled")) e.preventDefault(); });

    const slots = [];
    sels.forEach((s, i) => {
      if (i && spec.between) slots.push(el("span", { class: "muted", text: spec.between }));
      if (spec.max > 2) {
        // A small live preview of whoever sits in the slot.
        const c = el("canvas", { width: 34, height: 34, class: "slot-face", "aria-hidden": "true" });
        addPreview(c, () => lib.get(s.value)?.look, { scale: 0.3 });
        slots.push(el("span", { class: "car-slot" }, c, s));
      } else slots.push(s);
    });
    const resize = (d) => { remember([...sels.map((s) => s.value), ...(d > 0 ? [sels[sels.length - 1].value] : [])].slice(0, count + d)); renderPick(); };
    const sizeBtns = spec.max > spec.min ? [
      el("button", { type: "button", class: "btn small", text: "+ Car", "aria-label": `Add a ${game} slot`, disabled: count >= spec.max, onclick: () => resize(1) }),
      el("button", { type: "button", class: "btn small", text: "− Car", "aria-label": `Remove a ${game} slot`, disabled: count <= spec.min, onclick: () => resize(-1) }),
    ] : [];
    const seedGo = [el("label", { class: "muted" }, "Seed ", seed), go, err];
    rows.push(el("div", { class: `pick-game pick-${game}` }, el("h3", { text: titleCase(game) }),
      ...(sizeBtns.length
        ? [el("div", { class: "pick-row" }, ...slots, ...sizeBtns), el("div", { class: "pick-row" }, ...seedGo)]
        : [el("div", { class: "pick-row" }, ...slots, ...seedGo)])));
    update();
  }
  rows.push(el("p", { class: "muted small", text: "Links carry each Nyborg's build for the engine, plus its name and look for the drawing, so a shared link plays the same match for anyone. The same Nyborg can race more than once." }));
  box.replaceChildren(...rows);
}

// --- export / import -----------------------------------------------------------------

async function importText(text) {
  const parsed = parseImport(text);
  if (parsed.error) { banner(`Couldn't import: ${parsed.error}.`, "bad"); return; }
  let added = 0;
  const notes = [];
  for (const raw of parsed.items) {
    const r = sanitizeImported(engine, raw);
    if (r.error) { notes.push(r.error); continue; }
    let n = r.nyborg;
    const label = n.name;
    if (lib.get(n.id)) {
      const choice = await ask(`${lib.get(n.id).name} is already in your library.`, [
        { label: "Skip", value: "skip" }, { label: "Keep both", value: "both" }, { label: "Replace", value: "replace", kind: "primary" }]);
      if (choice === "skip") continue;
      if (choice === "both") n = { ...n, id: newId() };
    }
    if (safe(() => lib.save(n))) { added++; safe(() => lib.select(n.id)); }
    for (const note of r.notes) notes.push(`${label}: ${note}`);
  }
  renderAll();
  banner(`Imported ${added} Nyborg${added === 1 ? "" : "s"}.${notes.length ? " " + notes.join("; ") + "." : ""}`, notes.length ? "" : "ok");
}

// --- wiring --------------------------------------------------------------------------

function renderAll() {
  renderLibrary();
  renderEditor();
}

$("new").addEventListener("click", createNew);
$("dup").addEventListener("click", () => duplicate(lib.selected));
$("delete").addEventListener("click", () => remove(lib.selected));
$("export-one").addEventListener("click", () => { const n = selected(); if (n) download(fileNameFor(n), exportOne(n)); });
$("export-all").addEventListener("click", () => download(`nyborgs-${new Date().toISOString().slice(0, 10)}.json`, exportAll(lib.list)));
$("import").addEventListener("click", () => $("import-file").click());
$("import-file").addEventListener("change", async (e) => {
  const f = e.target.files?.[0];
  e.target.value = "";
  if (!f) return;
  if (f.size > 1024 * 1024) { banner("Couldn't import: file is larger than 1 MB.", "bad"); return; }
  await importText(await f.text());
});
$("name").addEventListener("change", commitName);
$("name").addEventListener("keydown", (e) => { if (e.key === "Enter") { e.preventDefault(); $("name").blur(); } });

addPreview($("preview"), () => selected()?.look, { scale: 2.3, flip: true });
if (storageBlocked) banner("This browser is blocking storage, so nothing is saved after you close the page. Export your Nyborgs to keep them.", "", "blocked");
if (seeded) banner(`Say hi to ${seeded.name}, your first Nyborg. Give it a look and a build, or make more with + New.`, "ok", "welcome");
renderAll();
requestAnimationFrame(loop);

// For headless checks.
window.__customizer = { lib, engine, games: GAMES, catalogs: CATALOGS, importText, selectTab: (g) => { gameTab = g; renderGameTabs(); }, drafts: () => drafts };
