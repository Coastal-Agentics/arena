// Native-vs-wasm parity check: re-simulate every pinned replay in
// engine-wasm/tests/parity/ with the committed web/pkg and compare with the manifest,
// whose expected values were recorded natively. The native side is
// engine-wasm/tests/parity.rs (cargo test). No browser, no npm dependencies.
//   node scripts/check-parity.mjs
// Exits 1 with a per-field diff if any fixture disagrees. The same checks run in a
// browser by importing checkFixtures() (docs/engine/determinism.md).

/** Fields compared with the manifest (all recomputed by wasm except format and seed). */
export const FIELDS = ["format", "seed", "tanks", "ticks", "outcome", "final_hash", "setup_hash"];

const show = (v) => JSON.stringify(v);

/**
 * Check every manifest entry. `texts` maps file name -> fixture JSON text;
 * `checkReplayJson` is the wasm export. Returns { lines, failures }.
 */
export function checkFixtures(manifest, texts, checkReplayJson) {
  const lines = [];
  const failures = [];
  for (const f of manifest.fixtures) {
    const text = texts[f.file];
    const diffs = [];
    if (text === undefined) {
      diffs.push(["file", "present", "missing"]);
    } else {
      let got;
      try {
        got = JSON.parse(checkReplayJson(text));
      } catch (e) {
        diffs.push(["load", "loads", String(e && e.message ? e.message : e)]);
      }
      if (got) {
        for (const k of FIELDS) if (show(got[k]) !== show(f[k])) diffs.push([k, show(f[k]), show(got[k])]);
        if (got.verify_error !== null) diffs.push(["Replay::verify", "ok", got.verify_error]);
        const bytes = new TextEncoder().encode(text).length;
        if (bytes !== f.bytes) diffs.push(["bytes", String(f.bytes), String(bytes)]);
      }
    }
    if (diffs.length === 0) {
      lines.push(`ok    ${f.file}  ${f.ticks} ticks  ${f.outcome ? f.outcome.reason : "unfinished"}  ${f.final_hash}`);
    } else {
      failures.push(f.file);
      lines.push(`FAIL  ${f.file}`);
      for (const [k, want, got] of diffs) lines.push(`        ${k}\n          manifest (native): ${want}\n          wasm:              ${got}`);
    }
  }
  return { lines, failures };
}

async function main() {
  const { readFileSync, readdirSync } = await import("node:fs");
  const { fileURLToPath } = await import("node:url");
  const path = (p) => fileURLToPath(new URL(p, import.meta.url));
  const { initSync, checkReplayJson, engineVersion } = await import("../web/pkg/engine_wasm.js");
  initSync({ module: readFileSync(path("../web/pkg/engine_wasm_bg.wasm")) });

  const dir = path("../engine-wasm/tests/parity/");
  const manifest = JSON.parse(readFileSync(dir + "manifest.json", "utf8"));
  const onDisk = readdirSync(dir).filter((n) => n.endsWith(".json") && n !== "manifest.json").sort();
  const listed = manifest.fixtures.map((f) => f.file).sort();
  const texts = Object.fromEntries(onDisk.map((n) => [n, readFileSync(dir + n, "utf8")]));

  const { lines, failures } = checkFixtures(manifest, texts, checkReplayJson);
  const unlisted = onDisk.filter((n) => !listed.includes(n));
  for (const n of unlisted) lines.push(`FAIL  ${n}\n        not in manifest.json (regenerate: ${manifest.regenerate})`);
  console.log(`check-parity: web/pkg (engine ${engineVersion()}) vs ${listed.length} pinned replays`);
  console.log(lines.join("\n"));
  const bad = failures.length + unlisted.length;
  if (bad > 0) {
    console.error(`check-parity: FAILED, ${bad} fixture(s) differ between native (manifest) and wasm (web/pkg).`);
    process.exit(1);
  }
  console.log(`check-parity: all ${listed.length} fixtures match`);
}

if (typeof process !== "undefined" && process.argv && process.argv[1]) {
  const { pathToFileURL } = await import("node:url");
  if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
}
