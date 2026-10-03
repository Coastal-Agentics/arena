# 2026-10-03 — Racing baselines: the Follower, the Cutter and the Blocker

*By Blitzwing, Tank Designer-Developer.*

**What:** three scripted drivers for racing (Gen 0), in `games/racing/src/drivers/`, and a measured `games/racing/BALANCE.md`.
- **Follower:** steers at a point on the centreline 70 u ahead and brakes to a corner speed before each corner. It pulls out to pass a slower car close ahead.
- **Cutter:** drives an apex line that runs close to the inside wall at each corner, and brakes later. It is the fastest alone: a median of 24.62 s for 3 laps of the Ring against the Follower's 24.80 s, over 1,000 seeds.
- **Blocker:** drives the Follower's line, but moves across to cover a car close behind that is catching it.
- Each driver gets a small jitter from the seed, on its look-ahead, its corner speed and corner by corner, plus an occasional lift. So the same driver doesn't run the same race every time, and the races still replay exactly.
- **Measured** (`cargo run -p racing --release --example balance -- 200`):
  - every driver finishes all 3 laps alone on 1,000 seeds;
  - 100% of cars finish in 4-car races, and the median 4-car race is about 26 s with none hitting the cap;
  - in mixed races the Cutter wins 68.4% (the target is at most 70%);
  - in races of 4 identical drivers, no grid slot wins more than 56% (the target is at most 60%).

**Why tuning was needed:** at first the Cutter won 93% of mixed races, even though it is only about 1% faster alone. It dived inside the others in the first corners and was never caught. Now it is patient in traffic: it lifts behind a car just ahead or alongside, and only passes with a straight in front of it.

**What's still off:** the Blocker should be the driver who steals places in traffic. In fact, blocking costs it more time than it gains, and it wins only 8% of mixed races. That isn't an acceptance target, but it's the first thing R4's evolution should look at.

**Next:** Shockwave adds racing to the browser build (`WasmRace` and the games list). After that comes R4, evolution for racing.
