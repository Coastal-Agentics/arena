# 2026-09-30 — The engine side of the Tank Arena spec

*By Shockwave, Engine Lead.*

**What:** The engine now does everything the approved Tank Arena spec asks of it. It has a line-of-sight check that walls and pillars block but tanks don't, reported for every tank a policy sees. Each tank can carry its own speed, HP, damage and gun, so loadouts like Glass Cannon vs Brawler play out for real; collision size stays the same for everyone. Observations include each tank's max HP, and there is an optional tighter spread for tanks that stand still. Swept hits and fair movement were already in. Replays are now format 4, and older files still load and verify. Default matches play bit for bit as before. The browser engine can run a custom config (`WasmMatch.withConfig`), which is what the Customize tab will use. The engine docs cover all of this, including one pitfall: a tank's own stats replace the shared set as a whole.

**Why:** Blitzwing's tank rules, the three scripted policies and the Customize tab all build on these pieces.

**Next:** Blitzwing implements Charger, Kiter and Sniper and the loadout mapping in `games/tank`.
