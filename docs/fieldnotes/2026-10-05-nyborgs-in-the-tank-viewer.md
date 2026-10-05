# 2026-10-05 — Nyborgs show up in the tank viewer

*By Blitzwing, Tank Designer-Developer.*

**What:** the live tank viewer under `web/` now draws each agent as a Nyborg by default, so you can watch them move mid-match.
- **Look:** a round cream head, two dot eyes and no mouth, with three chunky yarn strands in a team-primary colour (Cobalt for Blue, Yarn Red for Orange). The yarn has a soft outline, ply ticks and a light edge so it reads fuzzy and Muppet-like, not vegetable-like.
- **Facing:** the sprite is drawn facing right and flips when the hull heading's x component turns left, with a small deadzone so it doesn't flicker when the tank points nearly straight up or down.
- **Gameplay stays readable:** a short aim barrel still follows the turret, the HP bar sits above the head, and projectiles are unchanged. A tiny idle bob is driven by the sim tick and is cosmetic only.
- **Classic fallback:** a Sprites control (and `?sprites=classic`) brings back the old tank hull drawing. Nyborgs stay the default.

**Verified:** local viewer load with headless Chrome screenshots of both facings mid-match and no console errors; `cargo test -q --workspace` keeps the tank pins; `node scripts/check-viewer.mjs` and `node scripts/fieldnotes_index.mjs --check` pass. Only `web/` rendering plus this note and its card changed — no engine, wasm rebuild, or pin edits.

**Next:** the Customize tab can preview the same Nyborg look, and later a shared Nyborg profile can carry hair colour across arenas once the library lands.
