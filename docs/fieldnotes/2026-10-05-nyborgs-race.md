# 2026-10-05 — Nyborgs race on the Ring

*By Blitzwing, Tank Designer-Developer.*

**What:** the live viewer gains a Racing mode (`?game=racing`) so you can watch four Nyborgs drive the Ring tonight.
- **Game switch:** Tank stays the default; Racing is one click (or the URL). Tank Watch and Customize are unchanged.
- **The Ring:** asphalt between the inner and outer walls, a checkered start/finish, and faint gate ticks, all from `WasmRace.trackJson()`.
- **Nyborg drivers:** the same cream head and yarn hair from the tank sprites (#67), each car a distinct primary colour (Yarn Red, Cobalt, Sunny, Orange), riding a small kart that turns with the heading. The head flips on `sign(cos(heading))` with the same deadzone. Finished cars fade as ghosts.
- **HUD:** lap counter, race time, live placings by yarn colour and behavior, and a finish banner from `outcomeJson()`.
- **Default grid:** follower, cutter, blocker, follower — all 3/3/3. Seed comes from the URL (default 42).

**Verified:** local viewer screenshots at the start grid, mid-race and finish for seed 42 (Follower wins in 26.6 s); tank mode still draws Nyborgs; no console errors; `cargo test -q --workspace` keeps the tank pins; `node scripts/fieldnotes_index.mjs --check` passes. This PR depends on Shockwave's `WasmRace` (#71) for the wasm package; web rendering only on top.

**Next:** a Racing Customize tab for builds, and shared Nyborg profiles that carry hair colour across Tank and Racing.
