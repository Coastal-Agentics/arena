// The Nyborg app (docs/design/nyborg-app.md): one page at /arena/play/ with hash routes.
// Make your Nyborg, send it into an arena, come back and tweak it. Plain ES modules + the
// committed wasm, no build step. Looks and names never reach the engine.
import { loadEngine } from "./engine.js";
import { openLibrary, el } from "./ui.js";
import { startRouter } from "./router.js";
import home from "./screens/home.js";
import hatch from "./screens/hatch.js";
import setup from "./screens/setup.js";
import { viewerLink } from "../games/index.js";

const ROUTES = [
  { pattern: "", screen: home },
  { pattern: "new", screen: hatch },
  { pattern: "play/:game", screen: setup },
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

const engine = await loadEngine();
const { lib, storage, blocked } = openLibrary();
if (blocked) banner("This browser isn't letting the page save, so your Nyborgs last only until you close this tab.", "warn", "storage");
if (lib.migration?.error) banner("Your Nyborgs have new fuzzy hair, but this browser couldn't back up the old library, so the change isn't saved yet. Export from the Customizer to keep a copy.", "bad", "migration");
else if (lib.migration) banner(`New hair! ${lib.migration.count === 1 ? "Your Nyborg now has" : `Your ${lib.migration.count} Nyborgs now have`} a Shaggy mop. Pick another style any time, Classic yarn included.`, "ok", "migration");

const app = {
  engine, lib, storage, banner, safe,
  /** Where Start goes for a match: the viewer, with looks and names for the drawing only. */
  matchHref: (mod, match) => viewerLink(engine, mod, match),
  // Both fire hashchange, so the router shows the new screen; replace keeps Back sensible.
  go(hash, { replace = false } = {}) {
    if (replace) location.replace(hash); else location.hash = hash;
  },
};
startRouter(document.getElementById("screen"), ROUTES, app);

// For headless checks.
window.__app = { app, get hash() { return location.hash; } };
