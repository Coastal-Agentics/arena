// The house Nyborgs: a small named cast you play against (docs/design/nyborg-app.md, decision 4).
// Fixed looks, mixed across the hair styles, and a build for every arena (each spends the same
// 9 points as yours; tests/web/app.test.mjs checks them with the engine). They are ordinary
// format-2 Nyborgs that live in code, never in storage: ids start with "house-".
const at = "2026-10-11T00:00:00Z";
const tank = (id, attack, speed, defense) => ({ rules_version: 1, levels: { attack, speed, defense }, behavior: { kind: "scripted", id } });
const race = (id, power, top_speed, grip) => ({ rules_version: 1, levels: { power, top_speed, grip }, behavior: { kind: "scripted", id } });
const nyborg = (slug, name, tagline, look, builds) => ({
  format: 2, id: `house-${slug}`, name, created: at, updated: at,
  look: { strands: 3, accessories: [], ...look }, builds, house: true, tagline,
});

export const HOUSE = Object.freeze([
  nyborg("biscuit", "Biscuit", "Sunny and steady. Never leaves the racing line.",
    { hair_style: "pom", hair_color: "#F2C14E" },
    { racing: race("follower", 3, 3, 3), tank: tank("charger", 3, 3, 3) }),
  nyborg("noodle", "Noodle", "Old-school yarn, new-school moves. Dives for every gap.",
    { hair_style: "yarn", hair_color: "#EE7A68", strands: 4 },
    { racing: race("cutter", 4, 3, 2), tank: tank("kiter", 3, 4, 2) }),
  nyborg("captain-fuzz", "Captain Fuzz", "Sits wide and makes you go around.",
    { hair_style: "crest", hair_color: "#D4A23A" },
    { racing: race("blocker", 3, 2, 4), tank: tank("sniper", 4, 2, 3) }),
  nyborg("pepper", "Pepper", "All gas, no brakes, excellent glasses.",
    { hair_style: "mop", hair_color: "#B83246", accessories: ["glasses"] },
    { racing: race("cutter", 5, 2, 2), tank: tank("charger", 4, 1, 4) }),
  nyborg("mumble", "Mumble", "Can't see past the fringe. Drives by feel.",
    { hair_style: "fringe", hair_color: "#5AA9E6" },
    { racing: race("follower", 2, 3, 4), tank: tank("kiter", 3, 3, 3) }),
  nyborg("marigold", "Marigold", "Patient. Picks her moment.",
    { hair_style: "mop", hair_color: "#F08A3C", accessories: ["hat"] },
    { racing: race("blocker", 2, 2, 5), tank: tank("sniper", 3, 2, 4) }),
]);

export const isHouse = (id) => typeof id === "string" && id.startsWith("house-");
export const houseById = (id) => HOUSE.find((n) => n.id === id) || null;
