import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { MUJOCO_WASM_BYTES, HUD_KEY, isMobileLike, pickDpr, renderQuality, layoutFor, viewShift, progressTotal,
  progressFraction, formatMB, fetchWithProgress, loadFlag, saveFlag, createFpsMeter } from "../../web/tbd/phone.js";

test("isMobileLike: coarse pointer or short side under 600", () => {
  assert.equal(isMobileLike({ coarse: true, width: 1280, height: 800 }), true);
  assert.equal(isMobileLike({ width: 390, height: 844 }), true);
  assert.equal(isMobileLike({ width: 844, height: 390 }), true);
  assert.equal(isMobileLike({ width: 1280, height: 800 }), false);
  assert.equal(isMobileLike(), false);
});

test("pickDpr caps at 1.5 on phones and 2 on desktop", () => {
  assert.equal(pickDpr({ devicePixelRatio: 3, mobile: true }), 1.5);
  assert.equal(pickDpr({ devicePixelRatio: 1, mobile: true }), 1);
  assert.equal(pickDpr({ devicePixelRatio: 3, mobile: false }), 2);
  assert.equal(pickDpr({ devicePixelRatio: 1.25, mobile: false }), 1.25);
  assert.equal(pickDpr({ devicePixelRatio: NaN, mobile: false }), 1);
  assert.equal(pickDpr({ devicePixelRatio: 0 }), 1);
});

test("renderQuality: mobile drops shadows, antialias and PBR", () => {
  assert.deepEqual(renderQuality(true), { shadows: false, shadowMap: 0, antialias: false, lite: true });
  const d = renderQuality(false);
  assert.equal(d.shadows, true); assert.equal(d.antialias, true); assert.equal(d.lite, false); assert.ok(d.shadowMap >= 1024);
});

test("layoutFor: orientation, compact and rotate hint", () => {
  assert.deepEqual(layoutFor({ width: 390, height: 844, mobile: true }), { orientation: "portrait", compact: true, rotateHint: true });
  assert.deepEqual(layoutFor({ width: 844, height: 390, mobile: true }), { orientation: "landscape", compact: true, rotateHint: false });
  assert.deepEqual(layoutFor({ width: 1280, height: 800 }), { orientation: "landscape", compact: false, rotateHint: false });
  assert.equal(layoutFor({ width: 700, height: 900 }).rotateHint, false);   // desktop portrait window: no hint
});

test("viewShift centres the free band between top and bottom UI", () => {
  assert.equal(viewShift(844, 300, 200), -50);
  assert.equal(viewShift(844, 100, 300), 100);
  assert.equal(viewShift(844, 0, 0), 0);
  assert.equal(viewShift(400, 250, 200), 0);          // UI covers it all: no shift
  assert.equal(viewShift(844, undefined, -5), 0);
});

test("progressTotal: exact only when not content-encoded", () => {
  assert.deepEqual(progressTotal({ contentLength: "11201302" }), { total: 11201302, exact: true });
  assert.deepEqual(progressTotal({ contentLength: "2696116", contentEncoding: "br" }), { total: MUJOCO_WASM_BYTES, exact: false });
  assert.deepEqual(progressTotal({ contentLength: "500", contentEncoding: "identity" }), { total: 500, exact: true });
  assert.deepEqual(progressTotal({ contentLength: null, approx: 1000 }), { total: 1000, exact: false });
  assert.deepEqual(progressTotal({ contentLength: "abc", approx: 1000 }), { total: 1000, exact: false });
  assert.deepEqual(progressTotal({ contentLength: "0", approx: 1000 }), { total: 1000, exact: false });
});

test("progressFraction clamps to [0, 0.99] and formatMB", () => {
  assert.equal(progressFraction(50, 100), 0.5);
  assert.equal(progressFraction(100, 100), 0.99);
  assert.equal(progressFraction(200, 100), 0.99);
  assert.equal(progressFraction(-1, 100), 0);
  assert.equal(progressFraction(5, 0), 0);
  assert.equal(formatMB(11201302), "11.2 MB");
});

function fakeFetch(chunks, headers = {}, { stream = true, ok = true, status = 200 } = {}) {
  const h = new Map(Object.entries(headers).map(([k, v]) => [k.toLowerCase(), String(v)]));
  return async () => ({
    ok, status,
    headers: { get: k => h.get(k.toLowerCase()) ?? null },
    body: stream ? new ReadableStream({ start(c) { for (const x of chunks) c.enqueue(Uint8Array.from(x)); c.close(); } }) : null,
    arrayBuffer: async () => Uint8Array.from(chunks.flat()).buffer,
  });
}

test("fetchWithProgress streams with an exact content-length", async () => {
  const seen = [];
  const out = await fetchWithProgress("x", p => seen.push(p), { fetchImpl: fakeFetch([[1, 2], [3, 4, 5], [6]], { "content-length": 6 }) });
  assert.deepEqual([...out], [1, 2, 3, 4, 5, 6]);
  assert.deepEqual(seen.map(p => p.loaded), [2, 5, 6, 6]);
  assert.ok(seen.slice(0, -1).every(p => p.exact && p.total === 6 && p.fraction < 1));
  assert.ok(seen[1].fraction > seen[0].fraction);
  assert.deepEqual(seen.at(-1), { loaded: 6, total: 6, exact: true, fraction: 1 });
});

test("fetchWithProgress ignores encoded length, uses approx, grows the total if exceeded", async () => {
  const seen = [];
  const out = await fetchWithProgress("x", p => seen.push(p),
    { approx: 4, fetchImpl: fakeFetch([[1, 2, 3], [4, 5, 6], [7]], { "content-length": 2, "content-encoding": "br" }) });
  assert.equal(out.length, 7);
  assert.equal(seen[0].total, 4); assert.equal(seen[0].exact, false);
  assert.ok(seen[1].total >= 6);                       // grew past the approximate size
  assert.ok(seen.slice(0, -1).every(p => p.fraction <= 0.99));
  assert.equal(seen.at(-1).fraction, 1);
});

test("fetchWithProgress falls back to one shot without a stream, and throws on HTTP errors", async () => {
  const seen = [];
  const out = await fetchWithProgress("x", p => seen.push(p), { fetchImpl: fakeFetch([[9, 8, 7]], {}, { stream: false }) });
  assert.deepEqual([...out], [9, 8, 7]);
  assert.deepEqual(seen, [{ loaded: 3, total: 3, exact: true, fraction: 1 }]);
  await assert.rejects(fetchWithProgress("x", () => {}, { fetchImpl: fakeFetch([], {}, { ok: false, status: 404 }) }), /HTTP 404/);
});

test("loadFlag / saveFlag round-trip and survive a throwing or missing storage", () => {
  const m = new Map(); const st = { getItem: k => m.get(k) ?? null, setItem: (k, v) => m.set(k, v) };
  assert.equal(loadFlag(st, HUD_KEY), false);
  saveFlag(st, HUD_KEY, true); assert.equal(loadFlag(st, HUD_KEY), true);
  saveFlag(st, HUD_KEY, false); assert.equal(loadFlag(st, HUD_KEY), false);
  const bad = { getItem() { throw new Error("denied"); }, setItem() { throw new Error("denied"); } };
  assert.equal(loadFlag(bad, HUD_KEY), false); assert.doesNotThrow(() => saveFlag(bad, HUD_KEY, true));
  assert.equal(loadFlag(undefined, HUD_KEY), false); assert.doesNotThrow(() => saveFlag(null, HUD_KEY, true));
});

test("createFpsMeter measures a rolling window", () => {
  const f = createFpsMeter(1000);
  assert.equal(f.fps, 0);
  for (let i = 0; i <= 120; i++) f.tick(i * (1000 / 60));
  assert.ok(Math.abs(f.fps - 60) < 0.5, String(f.fps));
  for (let i = 1; i <= 60; i++) f.tick(2000 + i * (1000 / 30));
  assert.ok(Math.abs(f.fps - 30) < 0.5, String(f.fps));
});

test("index.html: phone viewport, touch controls, progress bar, rotate hint; css fits the viewport", () => {
  const html = readFileSync(new URL("../../web/tbd/index.html", import.meta.url), "utf8");
  const css = readFileSync(new URL("../../web/tbd/tbd.css", import.meta.url), "utf8");
  assert.match(html, /name="viewport"[^>]*viewport-fit=cover/);
  assert.match(html, /user-scalable=no/);
  for (const id of ["stick", "hud-toggle", "hud-body", "progress-bar", "rotate", "rotate-x"]) assert.match(html, new RegExp(`id="${id}"`));
  for (const h of ["climb", "descend", "yawl", "yawr"]) assert.match(html, new RegExp(`data-hold="${h}"`));
  assert.match(html, /role="progressbar"/);
  assert.match(css, /100dvh/); assert.match(css, /safe-area-inset-bottom/);
  assert.match(css, /touch-action:\s*none/); assert.match(css, /overscroll-behavior:\s*none/);
});
