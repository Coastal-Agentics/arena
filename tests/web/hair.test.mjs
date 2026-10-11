// Node tests for the hair styles (web/nyborg-hair.js, drawn by drawNyborg): deterministic, seeded by id.
//   node --test "tests/web/**/*.test.mjs"
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { seedFrom, rng, HAIR_STYLES } from "../../web/nyborg-hair.js";
import { HAIR_PALETTE, drawNyborg } from "../../web/nyborg-draw.js";
const drawNyborgHair = drawNyborg;
const HAIR_OPTIONS = HAIR_STYLES.filter((s) => s.id !== "yarn");

/** A canvas stand-in that records every call (rounded), enough to compare two drawings. */
function recorder() {
  const log = [];
  let scale = 1;
  const round = (v) => (typeof v === "number" ? Math.round(v * 1000) / 1000 : v);
  const ctx = new Proxy({}, {
    get(_, k) {
      if (k === "log") return log;
      if (k === "getTransform") return () => ({ a: scale, b: 0 });
      if (k === "scale") return (x, y) => { scale *= Math.abs(x); log.push(["scale", round(x), round(y)]); };
      if (k === "save" || k === "restore") return () => log.push([k]);
      return (...args) => log.push([k, ...args.map(round)]);
    },
    set(_, k, v) { log.push(["set", k, round(v)]); return true; },
  });
  return ctx;
}
const draw = (opts) => { const c = recorder(); drawNyborgHair(c, opts); return JSON.stringify(c.log); };

test("seeds are stable across runs and machines", () => {
  assert.equal(seedFrom("nyborg"), seedFrom("nyborg"));
  assert.notEqual(seedFrom("pip"), seedFrom("juno"));
  assert.equal(seedFrom(""), 0x811c9dc5, "FNV-1a offset basis");
  const a = rng(42), b = rng(42);
  const seqA = Array.from({ length: 5 }, a), seqB = Array.from({ length: 5 }, b);
  assert.deepEqual(seqA, seqB);
  assert.ok(seqA.every((v) => v >= 0 && v < 1));
});

test("no Math.random in the hair or drawing code", () => {
  for (const f of ["nyborg-hair.js", "nyborg-draw.js"]) {
    const src = readFileSync(fileURLToPath(new URL(`../../web/${f}`, import.meta.url)), "utf8");
    assert.ok(!/Math\.random/.test(src.replace(/\/\/.*$/gm, "")), f);
  }
});

test("same Nyborg id draws the same hair; another id draws different tufts", () => {
  assert.deepEqual(HAIR_OPTIONS.map((o) => o.letter), ["A", "B", "C", "D"]);
  for (const { id: style } of HAIR_OPTIONS) {
    for (const scale of [2.3, 0.72, 0.42]) {
      const opts = { style, id: "3f2c-pip", hair: "#3F6FD8", scale, accessories: ["glasses", "hat"] };
      assert.equal(draw(opts), draw(opts), `${style} @ ${scale}`);
      assert.notEqual(draw(opts), draw({ ...opts, id: "9a1b-juno" }), `${style}: ids differ`);
    }
    // Every palette colour draws (tones are computed, never hard-coded per colour).
    for (const { hex } of HAIR_PALETTE) assert.ok(draw({ style, id: "x", hair: hex }).length > 1000);
  }
});

test("facing -1 mirrors the whole Nyborg, and viewer size uses the lighter detail level", () => {
  const right = JSON.parse(draw({ style: "crest", id: "pip", scale: 1, facing: 1 }));
  const left = JSON.parse(draw({ style: "crest", id: "pip", scale: 1, facing: -1 }));
  assert.deepEqual(right.find((c) => c[0] === "scale"), ["scale", 1, 1]);
  assert.deepEqual(left.find((c) => c[0] === "scale"), ["scale", -1, 1]);
  assert.equal(right.length, left.length, "same drawing, mirrored by the transform");
  const big = JSON.parse(draw({ style: "mop", id: "pip", scale: 2.3 })).length;
  const small = JSON.parse(draw({ style: "mop", id: "pip", scale: 0.42 })).length;
  assert.ok(small < big * 0.6, `viewer size skips fuzz and texture (${small} vs ${big} calls)`);
});

test("yarn keeps today's drawing as the reference", () => {
  assert.ok(draw({ style: "yarn", hair: "#D9534F", strands: 3 }).includes("quadraticCurveTo"));
});

test("the default style is the Shaggy mop; unknown styles fall back to it", () => {
  const base = { id: "pip", hair: "#D9534F", scale: 1 };
  assert.equal(draw(base), draw({ ...base, style: "mop" }));
  assert.equal(draw({ ...base, style: "broccoli" }), draw({ ...base, style: "mop" }));
  assert.deepEqual(HAIR_STYLES.map((s) => s.id), ["mop", "pom", "fringe", "crest", "yarn"]);
});
