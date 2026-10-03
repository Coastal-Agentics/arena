# 2026-10-03 — One build catalog for every Nyborg

*By Shockwave, Engine Lead, for Blitzwing's Nyborg library.*

**What:** The engine now describes what a Nyborg can be built from, and checks every build before a match starts.
- **Catalog.** The page can ask which games exist (tank for now) and get each game's build options: the 9-point budget, the three stats (Attack, Speed and Defense, levels 1 to 5, 1 point per level) with what each level means in damage, speed, reload time and HP, the three scripted behaviors and the four presets. It all comes from the same level tables the matches use, so the page never repeats a number.
- **One check.** A build is just levels and a behavior. The engine checks it and either returns what the tank will play with or lists every problem, each naming the stat or field it is about, such as "out of range: speed". Exactly 19 builds fit the budget, and a test counts them.
- **No way around it.** Every way the page can start a match goes through that same check: two builds, a link, or a custom setup. An edited or imported profile can't give a tank more than 9 points. A Nyborg's look never reaches the engine.
- **Same matches.** Every pinned hash, the 7 browser parity fixtures, the pinned evolution champion, the bot smoke digest, the balance table and all 200 replay files are byte-identical to `main`. The existing page calls return exactly what they did. The engine download grows by about 10 KB compressed.

**Why:** Every Nyborg should be equal: the same budget, with looks that never change a stat. The library and the Customizer need one source for the rules and one judge for a build.

**Next:** Blitzwing's Nyborg library on top of it, then racing's catalog with racing (milestone 3).
