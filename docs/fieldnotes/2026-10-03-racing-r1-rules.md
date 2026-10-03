# 2026-10-03 — Racing rules: cars, the Ring track, laps and the finish

*By Blitzwing, Tank Designer-Developer.*

**What:** a new crate, `games/racing`, holds the rules for game #2 (spec: `docs/design/racing.md`, approved by Nye today).
- **Track:** the "Ring" is a 9-point centreline, 120 u wide, with straight mitred walls. It has 9 checkpoint gates that must be crossed in order and forwards, and 4 grid slots behind the line. Tracks are data, so Ring is only the first one.
- **Cars:** a circle of radius 12 with throttle and steering. A car can't turn while standing still, and steers half as sharply at top speed. Sliding sideways is controlled by grip, and scraping a wall costs 40% of the car's speed. All the maths uses plain additions and multiplications plus the engine's heading table, so a race plays out the same everywhere.
- **Setup:** Power, Top speed and Grip, levels 1–5 on the tanks' 9-point budget. That gives the same 19 setups as the tanks. The build catalog uses the keys `power`, `top_speed` and `grip`, with rules version 1.
- **Race:** 3 laps. The race ends when every car is home, or 10 s after the winner finishes, with a 60 s cap. Places live in the race's own state. A finished car turns into a ghost, so it can't block the field.
- **Training view:** each car gets 43 numbers: rays to the walls, the next two gates, its place, and the 3 nearest cars. It answers with 2 controls: throttle and steer. Its reward is the race progress gained each tick (a lap is worth 1) plus a finishing bonus by place.
- **Tests (rules plus catalog):**
  - the same seed and inputs give byte-identical replays;
  - the tank pins are untouched;
  - reversing over the line or skipping gates earns nothing;
  - the rewards add up exactly;
  - across 1,000 races, no car ever ends a tick inside a wall or another car;
  - a car fast enough to jump through a wall in one tick is rejected.

**Why:** racing is the second game on the shared engine interface. It shows that one Nyborg can race as well as fight.

**Next:**
- R2: the three scripted drivers (Follower, Cutter, Blocker) and a racing BALANCE.md with measured numbers.
- Then Shockwave's `Finished` end reason and replay format 5, which the rules already have a slot for.
