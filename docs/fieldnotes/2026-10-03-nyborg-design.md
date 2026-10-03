# 2026-10-03 — Meet the Nyborgs (design for a gate)

*By Blitzwing (Tank Designer-Developer).*

**What:** A short design, `docs/design/nyborg.md`, and a mockup sheet, `docs/design/nyborg-mockup.svg` and `.png`, for the characters Nye calls Nyborgs.
- **Look:** a soft-toy character with a round head, two dot eyes, no mouth, and 2–4 chunky yarn strands for hair. The hair color (and maybe the strand count) is customizable, and there are cosmetic upgrades such as glasses and hats.
- **Palette:** a quiet cream head. The hair comes in primary colors (red, blue, yellow) and a few close shades, and the default is Yarn Red `#D9534F`.
- **Animation:** two 2D loops. Idle is 4 frames at 6 fps, and move is 6 frames at 10 fps, with the yarn strands flopping and trailing as the Nyborg bounces along. A Nyborg facing left is the same frame mirrored, with a small threshold so it doesn't flicker.
- **One identity:** a Nyborg keeps one profile (name, look, lineage) and has a build and policy for each arena. It rides a tank in Tank Arena and drives a car in Racing.

- **Revision 2 (Nye's feedback, 10:45 AM ET):** the hair became a fuzzy, floppy felt mop, every green is gone in favor of primary colors, and the face gained a wide Muppet-style mouth and catchlights in the eyes.
- **Revision 3 (Nye's feedback, 10:53 AM ET):** no mouth, so the face is just the two dot eyes. The hair is now 2–4 chunky yarn strands with ply lines and curled tips. The colors are unchanged, with Felt Red renamed to Yarn Red.

**Why:** Nye wants agents people can recognise and customize, and the same Nyborg should be able to train in any arena. Cosmetics stay in the viewer, so no match, replay, hash or evolution run changes.

**Next:** Nye's call on the open questions: how many accessories, earned or free upgrades, names above heads, and head on or beside the tank. After the gate comes the asset pipeline and a Customize preview (N1), alongside the tank-only viewer refactor (V1).
