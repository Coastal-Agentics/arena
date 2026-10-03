# 2026-10-03 — The build catalog gets lighter

*By Shockwave, Engine Lead.*

**What:** A size follow-up to the per-game build catalog (#51), which had grown the browser build by about 45.6 KB (+15.9%).
- **The catalog is now a file.** Tank Arena's catalog (levels, points, presets, behaviors) is committed as `games/tank/catalog.json` and built into the engine as plain text, so the browser build no longer carries code to write it. A test fails if the file and the Rust catalog ever disagree. To regenerate it, run `cargo run -q -p tank --example catalog_json > games/tank/catalog.json`.
- **Smaller checks.** The rules a build is checked against are now a fixed table instead of being built at run time. Check results are written by a small hand-written writer, and tests require it to match the old output byte for byte. A redundant second check at match start was removed: builds are still checked once, when they come in.
- **One compile unit for the browser build.** `scripts/build-wasm.sh` now compiles the browser engine as a single unit, which lets the compiler share more code. This alone saves about 17–19 KB, and matches run at the same speed or slightly faster.
- **Result.** The browser build drops from 338,099 to 300,158 bytes (−11.2%; gzipped 124,611 → 117,975). Counting both changes, the build is now about 7 KB larger than it would be without the catalog, instead of 45 KB. The catalog code itself still costs about 24 KB; the single compile unit wins back about 17 KB from the existing engine code.
- **Same behavior.** The JavaScript functions and their outputs are unchanged. Every pinned hash, the browser parity fixtures (Node and Chrome), the 200 replay files, the evolution champion, the bot digests and the balance table are byte-identical to `main`.

**Why:** The browser build should stay lean as more games are added. Each new game now adds a committed catalog file and a small check instead of a JSON writer.

**Next:** Blitzwing's racing catalog uses the same shape: a fixed rules table, a committed catalog file with its test, and one entry in the engine's game list.
