// Hash routes for the Nyborg app (docs/design/nyborg-app.md §3). Static Pages has no rewrites,
// so routes live after the #: they never 404 and work from any static server. Pure helpers
// (parseHash, href, matchRoute) are tested in Node; startRouter wires them to the page.

/** "#/play/racing?me=x" -> {path: ["play", "racing"], params: URLSearchParams("me=x")}. */
export function parseHash(hash) {
  const h = String(hash ?? "").replace(/^#\/?/, "");
  const at = h.indexOf("?");
  const p = at < 0 ? h : h.slice(0, at);
  const q = at < 0 ? "" : h.slice(at + 1);
  const path = p.split("/").filter(Boolean).map((s) => { try { return decodeURIComponent(s); } catch { return s; } });
  return { path, params: new URLSearchParams(q) };
}

/** "#/nyborg/<id>" from ["nyborg", id], plus an optional query string or URLSearchParams. */
export function href(path = [], query = "") {
  const p = path.map((s) => encodeURIComponent(s)).join("/");
  const q = String(query instanceof URLSearchParams ? query.toString() : query ?? "").replace(/^\?/, "");
  return `#/${p}${q ? `?${q}` : ""}`;
}

/** First route whose pattern ("nyborg/:id") fits the path, with its named args; else null. */
export function matchRoute(routes, path) {
  for (const route of routes) {
    const parts = route.pattern.split("/").filter(Boolean);
    if (parts.length !== path.length) continue;
    const args = {};
    if (parts.every((part, i) => (part.startsWith(":") ? ((args[part.slice(1)] = path[i]), true) : part === path[i]))) return { route, args };
  }
  return null;
}

/**
 * Mount the screen for the current hash into `outlet`, and again on every hash change. A screen
 * is {title, mount(el, {args, params, app}) -> optional cleanup}. Unknown routes go home.
 */
export function startRouter(outlet, routes, app, { win = window } = {}) {
  let cleanup = null;
  const show = () => {
    const { path, params } = parseHash(win.location.hash);
    const hit = matchRoute(routes, path) || matchRoute(routes, []);
    if (typeof cleanup === "function") cleanup();
    cleanup = null;
    outlet.replaceChildren();
    const screen = hit.route.screen;
    win.document.title = `${screen.title?.(hit.args, app) || "Your Nyborgs"} · Nyborgs · Coastal Agentics`;
    cleanup = screen.mount(outlet, { args: hit.args, params, app });
    win.scrollTo?.(0, 0);
    outlet.querySelector("h1")?.focus?.({ preventScroll: true });
  };
  win.addEventListener("hashchange", show);
  show();
  return { refresh: show };
}
