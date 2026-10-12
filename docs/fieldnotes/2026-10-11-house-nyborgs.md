# 2026-10-11 — Meet the house Nyborgs, and set up your match

*By Blitzwing /nyborg.*

**What:** milestone 2 of `docs/design/nyborg-app.md`. Your Nyborg now plays named opponents, set up on one screen for both arenas.
- **The house Nyborgs** (`web/app/house.js`), six of them, mixed across every hair style:
  - **Biscuit**: yellow Pom cloud. Sunny and steady. Never leaves the racing line. (follower; charger 3/3/3)
  - **Noodle**: coral Classic yarn, four strands. Old-school yarn, new-school moves. Dives for every gap. (cutter 4/3/2; kiter 3/4/2)
  - **Captain Fuzz**: mustard Tousled crest. Sits wide and makes you go around. (blocker 3/2/4; sniper 4/2/3)
  - **Pepper**: crimson Shaggy mop with glasses. All gas, no brakes, excellent glasses. (cutter 5/2/2; charger 4/1/4, the Brawler)
  - **Mumble**: sky Muppet fringe. Can't see past the fringe. Drives by feel. (follower 2/3/4; kiter 3/3/3)
  - **Marigold**: orange Shaggy mop with a hat. Patient. Picks her moment. (blocker 2/2/5; sniper 3/2/4)
  - Every build spends the same 9 points as yours, checked by the engine in the tests. They live in code, never in your storage.
- **Match setup** (`#/play/racing`, `#/play/tank`): Race ▶ and Tank ▶ on home open it with your Nyborg against the house: Biscuit, Noodle and Captain Fuzz on the Ring; Pepper in the Tank Arena. Each slot shows its Nyborg's face, build and tagline. **Swap** any slot for one of your Nyborgs or another house Nyborg, add or remove cars (2–4) on the Ring, and pick a seed or **🎲 Shuffle**. The hash keeps the setup, so reload and Back land where you were. **Start** opens the match in today's viewer with every Nyborg's look and name (M3 brings it inside the app).
- **Home** gets a strip with the house cast.

**Verified:** headless screenshots of home with the cast, race setup (Pip against Biscuit, Noodle and Captain Fuzz), a swapped grid racing in the viewer with names in the HUD, tank setup against Pepper, and setup at 390 px with no sideways scroll. No console errors. Node tests: 27 pass, including new cast checks (names, shape, mixed styles, engine-valid builds on the 9-point budget), lineup rules, and links that play the same race and tank match as a direct engine run. `cargo test -q --workspace`, `check-viewer.mjs`, `check-parity.mjs` and the fieldnotes check pass.

**Next:** watching inside the app, with a finish card (milestone 3).
