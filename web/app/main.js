// The Nyborg app (docs/design/nyborg-app.md): one page at /arena/play/ with hash routes.
// Make your Nyborg, send it into an arena, come back and tweak it. Plain ES modules + the
// committed wasm, no build step. Looks and names never reach the engine.
import { loadEngine } from "./engine.js";
import { openLibrary, el } from "./ui.js";
import { startRouter } from "./router.js";
import home from "./screens/home.js";
import hatch from "./screens/hatch.js";
import setup from "./screens/setup.js";
import watch from "./screens/watch.js";
import edit from "./screens/edit.js";
import backup from "./screens/backup.js";
import { href } from "./router.js";
import { matchQuery } from "./match.js";

const ROUTES = [
  { pattern: "", screen: home },
  { pattern: "new", screen: hatch },
  { pattern: "play/:game", screen: setup },
  { pattern: "watch", screen: watch }, // someone's match link: #/watch?<viewer query>
  { pattern: "watch/:game", screen: watch },
  { pattern: "nyborg/:id", screen: edit },
  { pattern: "backup", screen: backup },
];

const banners = document.getElementById("banners");
function banner(text, kind = "", id = "") {
  if (id && banners.querySelector(`[data-banner="${id}"]`)) return;
  const b = el("div", { class: `banner ${kind}`, "data-banner": id || null },
    el("span", { text }), el("button", { class: "btn small", type: "button", text: "Dismiss", onclick: () => b.remove() }));
  banners.append(b);
}

/** Run a storage write; on failure say so in a banner and return null. */
function safe(fn) {
  try { return fn(); } catch (e) {
    if (e && (e.name === "QuotaExceededError" || e.code === 22)) banner("Couldn't save: browser storage is full. Export your Nyborgs from the Customizer to keep them.", "bad", "quota");
    else banner(`Couldn't save: ${e.message || e}`, "bad");
    return null;
  }
}

// A toast with Undo for 10 s (deleting a Nyborg).
const toast = document.getElementById("toast");
let toastTimer = 0;
function undoable(text, undo) {
  clearTimeout(toastTimer);
  toast.replaceChildren(el("span", { text }), el("button", { class: "btn small", type: "button", text: "Undo", onclick: () => {
    clearTimeout(toastTimer); toast.hidden = true; undo(); router.refresh();
  } }));
  toast.hidden = false;
  toastTimer = setTimeout(() => { toast.hidden = true; }, 10000);
}

const engine = await loadEngine();
const { lib, storage, blocked } = openLibrary();
if (blocked) banner("This browser isn't letting the page save, so your Nyborgs last only until you close this tab.", "warn", "storage");
if (lib.migration?.error) banner("Your Nyborgs have new fuzzy hair, but this browser couldn't back up the old library, so the change isn't saved yet. Export from the Customizer to keep a copy.", "bad", "migration");
else if (lib.migration) banner(`New hair! ${lib.migration.count === 1 ? "Your Nyborg now has" : `Your ${lib.migration.count} Nyborgs now have`} a Shaggy mop. Pick another style any time, Classic yarn included.`, "ok", "migration");

const app = {
  engine, lib, storage, banner, safe,
  /** Where Start goes: the watch screen, inside the app. */
  matchHref: (mod, match, ids) => href(["watch", mod.id], matchQuery(ids, match.seed)),
  /** Where "Tweak" goes for one of your Nyborgs. */
  tweakHref: (id) => href(["nyborg", id]),
  undoable,
  // Both fire hashchange, so the router shows the new screen; replace keeps Back sensible.
  go(hash, { replace = false } = {}) {
    if (replace) location.replace(hash); else location.hash = hash;
  },
};
const router = startRouter(document.getElementById("screen"), ROUTES, app);

// For headless checks.
window.__app = { app, get hash() { return location.hash; } };
