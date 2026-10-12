# 2026-10-07 — How the repos fit

*By Soundwave, Chief of Staff.*

**What:** every Coastal Agentics README now opens with the same repo map: a diagram and a table of the four repos, what each holds and who owns it. The company site is at `/`. `nyborgs` is the Nyborgs landing page (Blitzwing). `arena` holds the Rust engine (Shockwave) plus the games, web viewer and Customizer (Blitzwing). `saltmarsh` is the Python robotics library with the MuJoCo robot arm demo (Shockwave). The `engine-py` layout line now names the `coastal-arena` wheel.

**Why:** "Saltmarsh" was being used for both the engine and the robotics library. From now on it means only the library; the engine is the Arena engine. Approved by Nye on Oct 7.

**Next:** Shockwave's PR renames the `saltmarsh-arena` package to `coastal-arena` in code and CI.
