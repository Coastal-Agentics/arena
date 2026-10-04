# Nyborgs: character design

**Status:** Design draft for a gate (2026-10-03). Naming and design only. Nothing in the code is renamed, and nothing in the sim, the replays or evolution changes.
**Author:** Blitzwing (Tank Designer-Developer). Fits [game-system.md](game-system.md) and [viewer-multi-game.md](viewer-multi-game.md).

**In one line:** a Nyborg is a little soft-toy character with a round head, two dot eyes and 2–4 chunky strands of yarn for hair. The same Nyborg can train in any arena, driving a tank in Tank Arena and a car in Racing.

![Nyborg mockup](nyborg-mockup.png)

*Source: [nyborg-mockup.svg](nyborg-mockup.svg). This is a mockup, not final art. Revision 3 follows Nye's feedback: no mouth, and yarn-strand hair in primary colors (no green).*

## 1. Look
- **A round head** with a soft dark outline and a faint highlight. There is no body: the head *is* the Nyborg.
- **The face is just two dot eyes,** set a little apart at the middle of the head, each with a tiny white highlight. There is no mouth. When a Nyborg moves, its eyes shift slightly toward where it's going.
- **Yarn hair:** 2 to 4 individual strands of chunky yarn. Each one roots on the top of the head, curves up and out, and ends in a little curl.
  - **Strand:** a thick stroke with round ends and a soft outline.
  - **Texture:** thin darker diagonal lines along the strand suggest twisted ply, and a faint light line runs along one side.
  - Hair is the main thing you customize: its color, and possibly the strand count (question 5).
- **Flat shapes,** with no gradients. The only texture is the ply lines on the yarn. A Nyborg stays readable at the size of a tank on the 800 × 600 arena.

## 2. Customization (cosmetic only)
- **Hair color**, possibly the **strand count** (2, 3 or 4), and **upgrades** such as glasses and hats. A hat sits on the head, and the strands poke out from under the brim and over the crown.
- **Cosmetics never touch training.** They live apart from the training stats (the tank's 9-point loadout, racing's car setup). The sim, replays, state hashes and evolution never see them, so two Nyborgs with the same build and policy play exactly the same match.
- **Default palette:** the head stays a quiet cream. The hair uses primary colors (red, blue, yellow) plus a few close options. The default is Yarn Red, a softened red. There is no green.

| Part | Default | Other options |
|---|---|---|
| Head | Cream `#EADFCB` | Pebble `#D9D3C9`, Mist `#D2DBDF` |
| Hair, red | **Yarn Red `#D9534F`** (default) | Crimson `#B83246`, Coral `#EE7A68` |
| Hair, blue | Cobalt `#3F6FD8` | Sky `#5AA9E6`, Navy `#2F4A7A` |
| Hair, yellow | Sunny `#F2C14E` | Mustard `#D4A23A` |
| Hair, extras | — | Orange `#F08A3C`, Magenta `#C8459A` |
| Outline, eyes | Outline `#5E544B`, eyes `#2F2A26` | fixed |
| Upgrades (mockup) | glasses `#3B3632`; hat `#6E8BAE` with a `#F5F2EC` band | more later |

## 3. Animation (v1: 2D, two loops)
| Loop | Frames | fps | Length | What moves |
|---|---|---|---|---|
| **Idle** | 4 | 6 | 0.67 s | A gentle bob: a squash on frame 0 and a rise on frame 2. The strands sway ±4° and lag the head. |
| **Move** | 6 | 10 | 0.6 s | A bounce up to 8 px and a 6–7° lean forward. The strands flop and trail behind (up to about 18° of bend, most at the tips), then swing back. The eyes look ahead. |

- **Why these numbers:** 6 fps keeps the idle calm, because faster looks fidgety at 4 frames. 10 fps keeps the move lively without needing more frames, and 0.6 s per step reads well at tank speeds (120 u/s at Speed 3).
- **Facing:** sprites are drawn facing right. A Nyborg facing left is the same frame mirrored in x around its pivot.
- **No jitter when turning:** to stop it flipping back and forth, it only turns once the sideways speed passes about 0.5 u/tick in the new direction, and has stayed there for at least 6 ticks. Below that, it keeps its last facing.
- **Layers flip together.** The flip is one transform applied to the whole stack, so the hair, face and accessory stay lined up. A hat tilted to the right ends up tilted to the left, and that's intended.

## 4. Asset format (for the web/wasm viewer; the pipeline comes after the gate)
- **Separate layers:** base head, hair (the strands), eyes, accessory. All four are drawn on the same grid.
- **Hair color at runtime:** the hair sheet is grayscale and is tinted when it's drawn on the canvas. So there's one sheet for every color, and a new color is just a new hex value. The ply lines are drawn into the PNG. There is one hair sheet for each strand count.
- **Grid:**
  - each cell is 112 × 112 px, with the pivot at the feet (56, 104);
  - each sheet has one row per loop: row 0 idle (4 cells), row 1 move (6 cells);
  - a 2× sheet for sharp screens.
- **Manifest** `nyborg.json`:
  - the cell size and pivot;
  - for each loop, its row, frame count and fps;
  - for each frame, its offset and the attach points for `hat` (on the head, over the strand roots) and `glasses`, so an accessory follows the bob and the lean.
- **Files:**
  - SVG sources in the repo, under `web/assets/nyborg/src/`;
  - PNG sheets for runtime, under `web/assets/nyborg/` (`head.png`, `hair-2.png`, `hair-3.png`, `hair-4.png`, `eyes.png`, `acc-*.png` and `nyborg.json`).
  - The mockup's SVG already uses this layering. Each strand is plain strokes (outline, color, ply ticks, highlight) with no filters.
- **Viewer only.** Nyborgs are drawn on the viewer's canvas. Nothing about them reaches `engine`, `games/*` or `engine-wasm`'s sim calls, so determinism and the parity checks aren't touched.

## 5. One Nyborg, many arenas
| Part | Holds | Seen by the sim? |
|---|---|---|
| **Profile** | id, name, look (hair color, strand count, accessories), lineage and generation | No: viewer data only |
| **Build for each arena** | Tank Arena: Attack / Speed / Defense (9 points). Racing: Power / Top speed / Grip (9 points). Later arenas add their own. | Yes, as today's loadout |
| **Policy for each arena** | the scripted behavior or the evolved champion that arena trained | Yes, as today |

- **In an arena,** the Nyborg rides its body: its head sits on the tank's hull, or in the car, and turns with the facing rule above. Its name is shown only if Nye wants that (question 3).
- **The sim only sees the build and the policy:**
  - Tank links (`?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4`) stay exactly as they are.
  - The replay format stays 4, and nothing about the look reaches a hash.
  - The rules in [tank-refit.md](tank-refit.md) still hold.
- **Customize** grows from today's per-game schema to a per-Nyborg profile with a build for each arena, as in the viewer-multi-game.md "Direction" note.

**Where it fits.** These are viewer-only steps. Like V1 and V4, they sit outside the approved order (M1 → M2 → M4 → M3 → M5) and jump nothing:

| | What | When | Accepted when |
|---|---|---|---|
| **N1** | The asset pipeline (SVG → PNG sheets + manifest), and a Nyborg preview with idle and move in the Customize tab | after this gate, alongside V1 | Viewer checks pass unchanged; tank links and hashes unchanged |
| **N2** | The Nyborg's head on each tank in Watch, with facing and flip | after V1 | Same; draws stay off the sim's tick loop |
| **N3** | The Nyborg profile across arenas, plus look and Gen in the shared badge | with V4, then M5 for racing | One profile shows in both arenas; replays unchanged |

## 6. Open questions for Nye
1. **Accessories:** how many at once? One hat and one pair of glasses is the simple start.
2. **Upgrades:** are they earned through training (e.g. a hat at Gen 50), or free cosmetics from day one?
3. **Names:** should a Nyborg's name show above its head in Watch?
4. **Placement:** does the head sit *on* the tank or car, or float beside it?
5. **Hair:** is Yarn Red the right default color? And should the strand count (2, 3 or 4) be a cosmetic choice, or should one count be fixed for every Nyborg?
