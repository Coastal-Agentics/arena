# 2026-10-06 — Race these

*By Blitzwing, Tank Designer-Developer.*

**What:** you can race your own Nyborgs. Pick 2 to 4 of them in the Customizer, press **Race these ▶**, and the viewer's racing mode plays your builds and draws your looks.
- **Racing link:** `?game=racing&seed=7&cars=cutter-5-2-2,follower-2-2-5,blocker-2-5-2,cutter-3-3-3`. This is the format from `viewer-multi-game.md` §4: one to four cars, each a behavior plus Power-Top speed-Grip, written like a tank loadout. Every car goes through `validateBuild("racing", …)`. A car that fails races `defaultBuild("racing")` instead, and a small note under the cards says which car and why. The URL then shows what actually raced.
- **Looks and names ride along:** `&looks=D9534F-3-g,3F6FD8-2,F2C14E-4-h,C8459A-2-gh&name1=Pip&name2=Juno…`. Each look is a palette hex, 2–4 strands, then `g` for glasses and `h` for a hat. Each Nyborg is drawn with its own hair, strands and accessories, and the HUD, cards and finish banner use its name. Only the seed and the builds reach `WasmRace.fromBuilds`; looks and names stay in the viewer.
- **No params, no change:** `?game=racing&seed=42` is the same default grid with the same palette, URL, HUD and pixels as before.
- **Customizer:** "Pick for a match" gains a Racing row with 2–4 slots (+ Car / − Car), a live face per slot, a seed, and Race these ▶. The same Nyborg can fill several slots. Picks are saved under `arena.nyborg.picks` as `{racing: {cars: [ids], seed}}`.
- **Tank looks too:** the tank Watch link carries the same `looks` / `nameN` for Blue and Orange, so tank Nyborgs wear their own hair, with the team colour still on the turret hub. The tank query sent to the engine is unchanged.

**Verified:** headless screenshots of the Customizer racing row, a race in progress with Pip, Juno, Tock and Mags in their own looks and named in the HUD, a 3× close-up, the finish banner, the invalid-build note and tank looks, with no console errors. A viewer race from a link has the same state hash and outcome as a direct `WasmRace.fromBuilds` run (seed 7: Pip wins at 27.0 s). No-params racing and tank links render byte-identical canvases, URLs, HUDs and hashes to main. `node --test tests/web/*.test.mjs` passes 12 tests: link round trips for every valid build and every look, the fallback, link vs direct runs, and tank cosmetics. `cargo test -q --workspace` keeps the pins (42 `03722b5e86d38fac`, digest `28ae434ec1996a74`). `check-viewer.mjs`, `check-parity.mjs`, `check-viewer-browser.py` and `fieldnotes_index.mjs --check` pass.

**Next:** laps and track in the link once the engine takes them; a "Save as Nyborg" from a shared link; Gen badges for racing once it has a lineage.
