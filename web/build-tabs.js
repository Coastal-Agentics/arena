// Schema-driven build editing for the Customizer: one model for every game.
//
// Everything here reads the game's catalogJson(game): its stats (key, label, min, max,
// cost_per_level, values per level), budget, behaviors and presets. No game id, stat key or
// number is written in this file, so a third game that the engine adds to games() gets a
// tab with no Customizer change. tests/web/customizer.test.mjs proves it with a made-up
// "kite" catalog (four stats, budget 10, cost 2 on one stat).

const titleCase = (s) => s.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

/** Points a set of levels spends under a catalog (sum of level × cost_per_level). */
export function pointsSpent(catalog, levels) {
  let p = 0;
  for (const s of catalog.stats) p += (Number(levels?.[s.key]) || 0) * s.cost_per_level;
  return p;
}

/** The view model for one game's tab. `build` is what's on screen (maybe not yet valid). */
export function tabModel(catalog, build) {
  const spent = pointsSpent(catalog, build.levels);
  return {
    game: catalog.game,
    budget: catalog.budget,
    spent,
    remaining: catalog.budget - spent,
    stats: catalog.stats.map((s) => {
      const level = build.levels?.[s.key];
      return {
        key: s.key,
        label: s.label,
        min: s.min,
        max: s.max,
        cost: s.cost_per_level,
        level,
        readout: readoutFor(s, level),
        canDown: Number.isInteger(level) && level > s.min,
        canUp: Number.isInteger(level) && level < s.max,
      };
    }),
    behaviors: catalog.behaviors.map((id) => ({ id, label: titleCase(id) })),
    behavior: build.behavior?.kind === "scripted" ? build.behavior.id : null,
    presets: catalog.presets.map((p) => ({ id: p.id, label: p.label, levels: p.levels, on: sameLevels(catalog, p.levels, build.levels) })),
  };
}

/** "24 damage", "120 max speed · 45 fire cooldown": the resolved numbers for a level. */
export function readoutFor(stat, level) {
  const v = stat.values?.find((x) => x.level === level);
  if (!v) return "";
  return Object.entries(v).filter(([k]) => k !== "level").map(([k, n]) => `${n} ${k.replace(/_/g, " ")}`).join(" · ");
}

function sameLevels(catalog, a, b) {
  return catalog.stats.every((s) => a?.[s.key] === b?.[s.key]);
}

/** Pure edits: each returns a new build. Levels stay inside min..max; the budget is the validator's job. */
export function setLevel(catalog, build, key, level) {
  const s = catalog.stats.find((x) => x.key === key);
  if (!s) return build;
  const l = Math.max(s.min, Math.min(s.max, level));
  return { ...build, levels: { ...build.levels, [key]: l } };
}
export const applyPreset = (build, preset) => ({ ...build, levels: { ...preset.levels } });
export const setBehavior = (build, id) => ({ ...build, behavior: { kind: "scripted", id } });

/** A short line for library cards: "Kiter 4/3/2". */
export function summaryLine(catalog, build) {
  const beh = build.behavior?.kind === "scripted" ? titleCase(build.behavior.id) : "Champion";
  return `${beh} ${catalog.stats.map((s) => build.levels?.[s.key] ?? "?").join("/")}`;
}

/** Friendly words for validateBuild's keyed errors. `levels` are the ones on screen. */
export function friendlyErrors(catalog, result, levels) {
  if (!result || result.ok) return [];
  const label = (key) => catalog.stats.find((s) => s.key === key)?.label || titleCase(key || "build");
  return result.errors.map((e) => {
    switch (e.code) {
      case "over_budget": return `That's more than ${catalog.budget} points. Take a point off a stat.`;
      case "under_budget": {
        const left = catalog.budget - pointsSpent(catalog, levels);
        return `${left} ${left === 1 ? "point" : "points"} left to spend. Every Nyborg uses exactly ${catalog.budget}.`;
      }
      case "out_of_range": {
        const s = catalog.stats.find((x) => x.key === e.key);
        return s ? `${s.label} must be between ${s.min} and ${s.max}.` : `${label(e.key)} is out of range.`;
      }
      case "unknown_key": return `"${e.key}" isn't a stat in this game.`;
      case "unknown_behavior": return "Pick a behavior from the list.";
      case "rules_version_mismatch": return "This game's rules changed since this build was made. Rebuild with defaults to keep playing.";
      case "wrong_game": return "This build is for a different game.";
      case "invalid_json": return "This build couldn't be read.";
      default: return `${label(e.key)}: ${e.code.replace(/_/g, " ")}.`;
    }
  });
}
