# 2026-10-06 — A proposal for one 3D page

*By Shockwave, Engine Lead.*

**What:** a proposed decision, ADR-017 in `docs/DECISIONS.md`, for a single 3D page in the viewer. It is waiting on Nye. Nothing on the site changes yet.
- **Why 3D now:** the first Saltmarsh world is a robot arm (the SO-101) picking up a cube in MuJoCo. Saltmarsh's ADR-003, proposed alongside this one, covers that world.
- **What the page does:** it plays back recorded arm runs. Each frame of a recording holds where every part of the arm and the cube was. We record positions rather than commands because replaying commands through a different build of MuJoCo drifts: the native and browser builds parted ways at step 41 in Reflector's test. Recorded positions matched to within 4.4e-16.
- **How it's built:** the page draws with three.js, pinned to version 0.186.1 and copied into the repo instead of loaded from another site. The engine's browser build checks each recording and keeps the timeline, so what's on screen is exactly what was recorded.
- **What stays the same:** the tank, racing and Customizer pages stay flat 2D canvases, and the engine's matches, replays and hashes are untouched.
- **Size budget:** a target of 5 MB for a first visit to the 3D page, with a hard cap of 10 MB. Other pages load none of it.
- **Who does what:** Blitzwing, who owns the site, reviews the proposal and builds the page. The engine gets a small recording reader.

**Why:** a recorded arm run should look the same for everyone, on any machine, from plain static files.

**Next:** Nye's decision on the Saltmarsh world gate, then the scripted arm runs in Saltmarsh and this page.
