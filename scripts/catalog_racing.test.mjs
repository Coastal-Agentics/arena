// Racing's build-catalog calls in the committed web/pkg: games(), catalogJson("racing"),
// defaultBuild("racing") and validateBuild("racing", ...), plus wrong_game across games.
//   node --test scripts/catalog_racing.test.mjs
// The same cases run natively in engine-wasm's `build_exports_wrap_the_racing_catalog`
// (cargo test). No npm dependencies.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const path = (p) => new URL(p, import.meta.url);
const m = await import("../web/pkg/engine_wasm.js");
m.initSync({ module: readFileSync(path("../web/pkg/engine_wasm_bg.wasm")) });

const build = (levels, behavior, extra = {}) =>
  JSON.stringify({ ...extra, rules_version: 1, levels, behavior });
const scripted = (id) => ({ kind: "scripted", id });
const BALANCED = { power: 3, top_speed: 3, grip: 3 };
const errors = (...pairs) =>
  JSON.stringify({ ok: false, errors: pairs.map(([code, key]) => ({ code, key })) });
const validate = (json, game = "racing") => m.validateBuild(game, json);

test("games() lists tank then racing", () => {
  assert.equal(
    m.games(),
    '[{"game":"tank","rules_version":1},{"game":"racing","rules_version":1}]',
  );
});

test("catalogJson and defaultBuild return the committed racing JSON", () => {
  const committed = readFileSync(path("../games/racing/catalog.json"), "utf8");
  assert.equal(m.catalogJson("racing"), committed);
  const c = JSON.parse(committed);
  assert.equal(c.game, "racing");
  assert.equal(c.budget, 9);
  assert.deepEqual(c.stats.map((s) => s.key), ["power", "top_speed", "grip"]);
  assert.deepEqual(c.behaviors, ["follower", "cutter", "blocker"]);
  assert.equal(
    m.defaultBuild("racing"),
    '{"rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"scripted","id":"follower"}}',
  );
  assert.deepEqual(JSON.parse(m.defaultBuild("racing")), c.default_build);
});

test("tank's catalog calls are unchanged", () => {
  assert.equal(m.catalogJson("tank"), readFileSync(path("../games/tank/catalog.json"), "utf8"));
  assert.equal(
    m.defaultBuild("tank"),
    '{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"scripted","id":"charger"}}',
  );
  assert.match(validate(m.defaultBuild("tank"), "tank"), /^\{"ok":true,"game":"tank",/);
});

test("valid racing builds resolve to car params", () => {
  assert.equal(
    validate(m.defaultBuild("racing")),
    '{"ok":true,"game":"racing","rules_version":1,"levels":{"power":3,"top_speed":3,"grip":3},"behavior":{"kind":"scripted","id":"follower"},"points":9,"params":{"power":240.0,"top_speed":240.0,"grip":0.25}}',
  );
  // Out-of-order levels come back in stat order; a "game" field naming racing and an
  // unknown top-level field (a Nyborg's look) are accepted and dropped.
  assert.equal(
    validate(build({ grip: 1, power: 4, top_speed: 4 }, scripted("blocker"), { game: "racing", look: { paint: "red" } })),
    '{"ok":true,"game":"racing","rules_version":1,"levels":{"power":4,"top_speed":4,"grip":1},"behavior":{"kind":"scripted","id":"blocker"},"points":9,"params":{"power":280.0,"top_speed":255.0,"grip":0.15}}',
  );
  const champ = JSON.parse(validate(build(BALANCED, { kind: "champion", ref: "racing/nightly/gen-1" })));
  assert.equal(champ.ok, true);
  assert.deepEqual(champ.behavior, { kind: "champion", ref: "racing/nightly/gen-1" });
});

test("each error code, with its key", () => {
  const cases = [
    ["{not json", errors(["invalid_json", ""])],
    ["[1,2]", errors(["invalid_json", ""])],
    ['{"game":"tank","rules_version":1}', errors(["wrong_game", "game"])],
    ['{"rules_version":2}', errors(["rules_version_mismatch", "rules_version"])],
    [build({ ...BALANCED, nitro: 1 }, scripted("follower")), errors(["unknown_key", "nitro"])],
    [build({ power: 6, top_speed: 2, grip: 1 }, scripted("follower")), errors(["out_of_range", "power"])],
    [build({ power: 5, top_speed: 3, grip: 3 }, scripted("follower")), errors(["over_budget", "levels"])],
    [build({ power: 1, top_speed: 1, grip: 1 }, scripted("follower")), errors(["under_budget", "levels"])],
    [build(BALANCED, scripted("kiter")), errors(["unknown_behavior", "behavior"])],
    [build(BALANCED, { kind: "champion", ref: " " }), errors(["unknown_behavior", "behavior"])],
    // Several problems are all reported, in order.
    [
      build({ power: 0, top_speed: 3, grip: 3, nitro: 1 }, { kind: "scripted" }),
      errors(["unknown_key", "nitro"], ["out_of_range", "power"], ["unknown_behavior", "behavior"]),
    ],
  ];
  for (const [json, want] of cases) assert.equal(validate(json), want, json);
});

test("wrong_game across games", () => {
  const wrong = errors(["wrong_game", "game"]);
  const racingDefault = m.defaultBuild("racing");
  const tankDefault = m.defaultBuild("tank");
  // A build that names the other game.
  assert.equal(validate(tankDefault.replace("{", '{"game":"tank",'), "racing"), wrong);
  assert.equal(validate(racingDefault.replace("{", '{"game":"racing",'), "tank"), wrong);
  // A game nobody registered.
  assert.equal(validate(racingDefault, "chess"), wrong);
  assert.equal(validate(racingDefault, ""), wrong);
  assert.throws(() => m.catalogJson("chess"), /unknown game "chess"/);
  assert.throws(() => m.defaultBuild("Racing"), /unknown game "Racing"/);
  // An unlabeled build checked against the other game fails on its keys, not silently.
  assert.equal(
    validate(tankDefault, "racing"),
    errors(
      ["unknown_key", "attack"], ["unknown_key", "speed"], ["unknown_key", "defense"],
      ["out_of_range", "power"], ["out_of_range", "top_speed"], ["out_of_range", "grip"],
      ["unknown_behavior", "behavior"],
    ),
  );
});

test("fromBuilds stays Tank Arena only", () => {
  const b = m.defaultBuild("racing");
  assert.throws(() => m.WasmMatch.fromBuilds("racing", "1", b, b), /blue: invalid build \[\{"code":"wrong_game","key":"game"\}\]/);
});
