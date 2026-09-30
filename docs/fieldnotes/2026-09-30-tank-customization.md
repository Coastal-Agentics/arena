# 2026-09-30 — Tank customization in the spec

*By Blitzwing, Tank Designer-Developer.*

**What:** Nye decided tanks should be customizable, so the Tank Arena spec now has it. Every tank spends exactly 9 points across attack, speed and defense, 1 to 5 each: 19 possible builds, with 3/3/3 as the default and presets like Glass Cannon (5/3/1), Brawler (4/1/4) and Scout (2/5/2). The arena viewer gets a Customize tab next to Watch: pick each tank's behavior, drag a point in a triangle to set its build, see the real numbers (damage, speed, HP, hits to kill), and press Watch. The seed and both builds go in the link, so a shared link replays the same match. Each tank card also says how its behavior was made: "Scripted" today, and generation, win rate and a fitness sparkline once evolution arrives. An item kit (heavy shell, smoke, armor) is deferred to its own spec.

**Why:** Choosing a build and watching it play out is the fun part. A fixed budget keeps every build fair, and 3/3/3 keeps the default match exactly as it was.

**What's next:** Nye's GATE-002 review. The engine needs per-tank stats (engine ask #5) before the Customize tab can be built.

## Card: Tank customization spec
- **Artifact:** `games/tank/SPEC.md` (customization sections)
- **Made by:** Blitzwing (Tank Designer-Developer) / pending Nye's gate approval
- **From:** Nye's customization decision and the existing tank stat table · seeds n/a · commit: PR #4
- **Hours / compute:** one design pass; CPU only
- **Reward or fitness function:** n/a (specification)
- **License:** MIT
