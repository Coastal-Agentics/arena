# 2026-10-03 — Tank Arena and Racing as Python training environments

*By Shockwave, Engine Lead.*

**What:** Milestone 4 in the game-system design: the Python bridge, `engine-py`, which builds a package named `saltmarsh-arena`.
- **Both games, one path.** A single Rust session runs any game through its fixed-size number view. Tank Arena and Racing both go through it. A new game needs one small adapter: how its builds become a match, and its scripted drivers.
- **Three ways in.** A PettingZoo environment for several learning agents at once. A Gymnasium environment with one learner, where the other agents drive themselves in Rust. And a bare `FlatEnv` that shares memory with Rust, for the fastest loops. Matches start from the same builds the viewer uses, checked by the same catalog. Each environment can export its match as a replay.
- **Same matches as Rust.** A seeded episode played from Python produces byte for byte the replay of the same actions played natively, with the same final hash. Tests check this for both games, with every mix of learning and scripted agents. They also run PettingZoo's and Gymnasium's own conformance tests. One package installs on Python 3.10 and 3.14. A short Stable-Baselines3 training run works on both games.
- **Fast, and kept apart.** A step reads its actions from a buffer and writes observations back in place. Over a whole episode it allocates memory only when the match's action history grows. From Python, a bare step runs at about 1.2 million steps a second for a tank duel and 0.4 million for a four-car race. The bridge is its own build, so the engine, the browser build and the command-line runner don't include it. Their outputs are byte-identical to `main`.
- **A correction.** The merge title of #59, the catalog slim, says the browser build shrank by 10.4%. The final cut in that PR was 11.2% (338,099 to 300,158 bytes), as its field note says.

**Why:** Standard training tools can now train agents for both games. Their matches still replay exactly, in Rust or in the browser.

**Next:** Soundwave adds the package's CI workflow (the spec is in the PR). Publishing the package is a separate decision for Nye.
