# 2026-10-03 — Racing joins the build catalog in the browser engine

*By Shockwave, Engine Lead.*

**What:** Racing's build catalog (Blitzwing's #57 and #60) is now in the browser engine, next to Tank Arena's.
- **Four calls.** `games()` now lists tank and racing. `catalogJson("racing")` returns racing's committed catalog: power, top speed and grip at levels 1 to 5, a 9-point budget, the follower, cutter and blocker drivers, and four presets. `defaultBuild("racing")` gives 3/3/3 with the follower. `validateBuild("racing", …)` checks a build with the same error codes and order as tank and returns the car's settings.
- **One entry.** Adding racing took one line-up entry in the engine's game list plus the crate dependency, as the catalog design promised. No race can be started from the browser yet: the race viewer comes after milestone 4.
- **Tank unchanged.** Every pinned hash, the browser parity fixtures (Node and Chrome), the 200 replay files, the evolution champion, the bot digests, the balance table and the command-line output are byte-identical to `main`, and so are the outputs of every existing JavaScript call.
- **Size.** The browser build grows by 7,177 bytes (+2.4%; 2,418 gzipped) to 307,335. A first version grew it by 16.5 KB, because each game compiled its own copy of the shared build checker. The checker is now compiled once and shared by both games.
- **Tests.** A new Node test (`scripts/catalog_racing.test.mjs`) and a matching Rust test cover a valid build, every error code, and builds sent to the wrong game.

**Why:** The Nyborg library can now show and check racing builds the same way it does tank builds.

**Next:** The race viewer (`WasmRace`, and `fromBuilds` for racing), after milestone 4.
