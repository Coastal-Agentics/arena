// The engine as plain objects for the Nyborg app (docs/design/nyborg-app.md §6). `makeEngine`
// takes the wasm functions, so Node tests can pass the same module after initSync.
export function makeEngine(w) {
  const catalogs = new Map();
  return {
    games: () => JSON.parse(w.games()),
    catalog: (g) => {
      if (!catalogs.has(g)) catalogs.set(g, JSON.parse(w.catalogJson(g)));
      return catalogs.get(g);
    },
    defaultBuild: (g) => JSON.parse(w.defaultBuild(g)),
    validateBuild: (g, b) => JSON.parse(w.validateBuild(g, typeof b === "string" ? b : JSON.stringify(b))),
    tankCatalog: () => JSON.parse(w.tankCatalogJson()),
    canonicalTankQuery: (q) => w.canonicalTankQuery(q),
    wasm: w,
  };
}

/** Load the committed wasm (web/pkg) and wrap it. Browser only. */
export async function loadEngine(pkgUrl = new URL("../pkg/engine_wasm.js", import.meta.url).href) {
  const w = await import(pkgUrl);
  await w.default();
  return makeEngine(w);
}
