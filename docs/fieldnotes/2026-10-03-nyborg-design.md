# 2026-10-03 — Meet the Nyborgs (design for a gate)

*By Blitzwing (Tank Designer-Developer).*

**What:** A short design, `docs/design/nyborg.md`, and a mockup sheet, `docs/design/nyborg-mockup.svg` and `.png`, for the characters Nye calls Nyborgs.
- **Look:** a round head, dot eyes and a three-leaf tuft like a sprout. The tuft color is customizable, and there are cosmetic upgrades such as glasses and hats.
- **Palette:** quiet colors by default (Oat head, Moss tuft), with four bright tufts to choose from.
- **Animation:** two 2D loops. Idle is 4 frames at 6 fps, and move is 6 frames at 10 fps. A Nyborg facing left is the same frame mirrored, with a small threshold so it doesn't flicker.
- **One identity:** a Nyborg keeps one profile (name, look, lineage) and has a build and policy for each arena. It rides a tank in Tank Arena and drives a car in Racing.

**Why:** Nye wants agents people can recognise and customize, and the same Nyborg should be able to train in any arena. Cosmetics stay in the viewer, so no match, replay, hash or evolution run changes.

**Next:** Nye's call on the open questions: how many accessories, earned or free upgrades, names above heads, and head on or beside the tank. After the gate comes the asset pipeline and a Customize preview (N1), alongside the tank-only viewer refactor (V1).
