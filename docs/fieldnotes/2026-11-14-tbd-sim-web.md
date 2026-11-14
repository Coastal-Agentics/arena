# 2026-11-14 — TBD sim: a playable physics sim at /arena/tbd/

*By Blitzwing /nyborg.*

**What:** `/arena/tbd/` is a playable physics sim of a hybrid drive-and-fly vehicle. TBD is a working name. It runs MuJoCo 4.0.1 (WebAssembly) physics with three.js rendering. **The parameters are estimates, not measurements.** Masses, thrust, motor and servo numbers come from the BOM and CAD v0.2. You drive it on four wheels, unfold the ducts and fly with assisted hover, or run the autopilot stair hop. It works with keys, a gamepad, or touch (a joystick plus buttons) on a phone. The play hub links it from the subnav.

## Work log

**Brief (Soundwave, relaying Nye's approval, Nov 14):** port the box prototype into the arena web build at `web/tbd/`, link it from the play hub, make it work on phones, and keep the JS controller as the test reference behind a step interface so Shockwave's Rust/wasm step can drop in. One PR, not to merge without Nye's OK. Copy says "physics sim with estimated parameters". Keep public wording minimal and factual.

**Steps**
1. Read the prototype. It loads `model/tbd.xml` plus 13 STL meshes (about 0.8 MB), a vendored MuJoCo 4.0.1 (`mujoco.js` 295 KB plus `mujoco.wasm` **11.2 MB**) and a vendored three.js r187 (about 2.2 MB with two addons).
2. Hosting: the wasm is over the 10 MB line, so MuJoCo and three.js load from pinned jsDelivr URLs (`@mujoco/mujoco@4.0.1`, `three@0.187.0`) through an import map. I checked every pinned file against the prototype's vendored copy: all six SHA-256 hashes match. Only the model and meshes are committed. Every other path is relative and cache-busted with `?v=20261114a`.
3. Split the controller out of `main.js` into `web/tbd/controller.js`. It holds the flight controller, battery model, key, pad and touch mapping, the mode machine and the autopilot, all behind `reset(state)`, `press(action)` and `step(state, input) -> commands`. `main.js` only copies the commands into MuJoCo `ctrl` and steps the physics.
4. `controller-wasm.js` is the adapter for the Rust/wasm step. It's a documented stub: `?ctrl=wasm` falls back to JS. No Rust or wasm controller PR had landed (`gh pr list --search wasm` lists engine PRs only).
5. Phone: a left joystick (forward and turn on the ground, forward and yaw in flight) plus Climb, Fly/Land, Descend, Hop, Cam and Reset buttons. A compact two-column status panel and a wrapped header that keeps the estimated-parameters notice visible.
6. Tests: `tests/web/tbd.test.mjs` has 12 tests covering the math, mixer, altitude hold on a point-mass plant, battery, key, pad and touch mapping, the step interface, fly, land and fold, the low-battery landing, the autopilot phases, determinism, the wasm fallback and the page copy.

**What failed along the way**
- The first altitude-hold test asserted 1 cm after 5 s and saw 4 cm of overshoot from the integral term. Settling takes longer, so the test now runs 12 s and checks velocity too.
- The first headless run timed checks on the wall clock. SwiftShader runs the sim slower than real time in places, so the autopilot hadn't finished. The checks now wait on sim state.

**Verified**
- `node --test tests/web/*.test.mjs scripts/*.test.mjs`: everything passes, including the 12 new tests.
- Headless Chrome (SwiftShader), served under `/arena/`. At 1280×800 the hub link lands on `/arena/tbd/`, then drive, take off, climb, yaw, land and fold, and the autopilot hop ending on top of the stairs. At 390×844 with touch, the joystick drives, Fly plus Climb reaches 1.3 m, buttons are at least 48 px and the status panel clears the controls. No console errors and no failed requests.

**Not tried yet:** real phone touch hardware, a real gamepad, and first-load time of the 11 MB wasm on mobile data.

## Follow-up (Nov 14, afternoon): wasm controller wired for #132

**Brief:** Shockwave's engine PR [#132](https://github.com/Coastal-Agentics/arena/pull/132) (branch `engine/tbd-sim`) adds `WasmTbd` to `web/pkg`. Wire it into `controller-wasm.js`, keep JS as the reference and fallback, add a parity test that skips without the pkg, and compare against his native reference. Don't commit his `web/pkg` or other files.

**Model copies:** `assets/tbd/` (from #132) is the source of truth for `tbd.xml` and the meshes. `web/tbd/model/` stays a copy because Pages uploads only `web/`. Today the XML bodies match apart from the header comment, and all 13 STLs are byte-identical. A CI check that the two copies match can come later.

**Steps**
1. `controller-wasm.js` wraps `WasmTbd` behind the same interface as `createJsController()`:
   - **Loading:** it imports `../pkg/engine_wasm.js` dynamically. If the module is missing or has no `WasmTbd`, the page falls back to JS and the status panel says "JS controller".
   - **Each tick:** obs[20] is built from `sensordata[0..12]` plus the root qpos and qvel plus the pilot channels. The 9 commands go into `ctrl`, and `wind_force_n` goes into `xfrc_applied` on body `tbd`. The controller steps once per `mj_step`.
   - **Manual mode:** W/S and the joystick map to `pilot_x`. Turning maps to `pilot_yaw` with the engine's sign (positive speeds the right side). F sends the takeoff and land sentinels. The page sends the fold sentinel while landing, so it folds once down, as the JS controller does.
   - **G:** starts a fresh `stair_hop` game. When `over`, the page keeps the score and hands control back to a new `manual` game.
2. Built #132 in a separate worktree with `scripts/build-wasm.sh` (pinned toolchain, wasm-bindgen 0.2.100). The output is byte-identical to the `web/pkg` committed in #132. Then served `web/` with that pkg and ran headless hops through a `TBD.runHop()` hook, which steps physics without rendering.
3. Tests: 5 more in `tests/web/tbd.test.mjs`. Two cover the score formula and the adapter mapping. Three are parity tests against `WasmTbd` (manual skid steer, stair-hop phases and hover thrust, and the takeoff/land sentinels with wind), and they skip when `web/pkg` has no `WasmTbd`. Tolerances are loose because the next engine PR adds voltage sag, ram drag, ground effect and a slower fold servo.

**Results** (seed 1, stair hop, browser MuJoCo 4.0.1):

| run | score | completed at | deck touchdown* | peak tilt | min clearance | battery used |
|---|---|---|---|---|---|---|
| native reference (MuJoCo 3.17, wind) | 669.15 | 10.986 s | — | 18.9° | 0.30 m | — |
| browser, wasm controller, wind | 669.15 | 10.986 s | 8.486 s | 18.88° | 0.300 m | 5.07 % |
| browser, wasm controller, no wind | 678.31 | 10.598 s | 8.098 s | 19.58° | 0.299 m | 4.77 % |
| browser, JS controller, no wind | 694.60 | 9.994 s | 8.094 s | 19.31° | 0.299 m | 4.76 % |

\* Touchdown means the landing is confirmed on the deck, 0.4 s after contact.

The wasm run with wind matches the native reference to every printed digit. The JS reference lands within 4 ms of the wasm run with the same wind setting. Its score is higher only because its script has no 0.6 s settle phase, so the clock stops sooner.

**What failed along the way:** the first parity test compared thrust on every tick. On the deploy → climb transition tick the engine already flies while the JS reference waits one tick, so the test now skips the thrust check on transition ticks. My first clearance expectation used the wrong step: x = 1.0 is on step 2, which is 0.34 m.

**Default controller (superseded below):** `?ctrl=wasm` stayed **opt-in** at this point. The engine's stair hop is reliable and matches native. But the engine's manual mode has no climb, descend or strafe channel (it holds 0.6 m), so the Space/Shift keys and the Climb/Descend touch buttons would do nothing by default. JS stays the default and the reference until manual mode gets those channels.

**Verified:** `node --test tests/web/*.test.mjs scripts/*.test.mjs` passes 85 of 85 with #132's pkg in place. With main's pkg it's 82 passed and 3 skipped. Headless runs: the earlier desktop and phone checks still pass on JS. `?ctrl=wasm` without `WasmTbd` shows "JS controller" with no console errors. With #132's pkg it runs the stair hop with wind, and manual mode drives, takes off, lands and folds.

## Follow-up (Nov 14, later): #133 physics and manual channels

**Brief:** Shockwave's [#133](https://github.com/Coastal-Agentics/arena/pull/133) is stacked on #132. It adds `pilot_y` (strafe) and `pilot_z` (climb rate), so `tbdObsLen()` is now 22, though 20 still steps. It also uses the JS mode names, accepts a Number, BigInt or string seed, and adds aero force and torque. Estimated physics is on by default: voltage sag, ram drag, ground effect and a 0.19 s fold. His native run is 529.95 at 15.360 s.

**Steps**
1. **Obs and controls:** the adapter sizes obs from `tbdObsLen()`.
   - On a 22-slot build, Space/Shift and the Climb/Descend touch buttons map to `pilot_z`. A/D, the gamepad left stick X and the touch joystick X map to `pilot_y` in flight (strafe), while Q/E, the right stick and the new Turn L/R touch buttons map to `pilot_yaw`.
   - On a 20-slot #132 build, A/D yaw as before and there is no climb.
   - F while landing returns to flight on #133.
2. **Forces:** `xfrc_applied` on body `tbd` gets force = `wind_force_n + aero_force_n` and torque = `aero_torque_nm`, both world frame.
3. **Mode names:** #133 reports the JS names, so no translation is needed. The adapter keeps only an alias for #132's `unfolding`/`folding` so a #132 build still labels correctly.
4. **Stow workaround:** kept on both builds. The engine never stows by itself, and the JS controller folds automatically once down, so the page holds the stow signal while landing. On #133 that takes effect on the touchdown tick. I kept the seed as a BigInt because #132 rejects a Number.
5. **Default:** wasm runs when `../pkg` exports `WasmTbd` with `tbdObsLen() >= 22`; otherwise JS. `?ctrl=js` forces JS. `?ctrl=wasm` also allows a 20-slot build, and the status panel then reads "no climb/strafe". JS stays the test reference. The panel also shows pack voltage when the engine reports it.
6. **Tests:** parity tests pass on both builds and skip without a pkg.
   - The control-law check runs with #133's physics switches off; the switches don't exist on #132, so the adapter doesn't send them there.
   - A defaults-on check only bounds the hover-thrust gap (±0.25 per duct, in normalised thrust).
   - A #133-only test checks that climb and descend change the collective, that strafe rolls the right way, and that F aborts a landing.
   - Totals: 87/87 pass on #133, 86 pass + 1 skip on #132, and 82 pass + 5 skip on main's pkg.

**Results** (seed 1, stair hop, browser MuJoCo 4.0.1; #132 and #133 built locally with `scripts/build-wasm.sh`):

| run | score | completed at | deck touchdown | peak tilt | min clearance | battery used |
|---|---|---|---|---|---|---|
| native #133 reference (MuJoCo 3.17, wind) | 529.95 | 15.360 s | — | — | — | — |
| browser #133 wasm, wind | 529.87 | 15.360 s | 12.860 s | 18.12° | 0.271 m | 8.48 % |
| browser #133 wasm, no wind | 672.76 | 10.544 s | 8.044 s | 19.63° | 0.263 m | 4.71 % |
| browser #132 wasm, wind (earlier) | 669.15 | 10.986 s | 8.486 s | 18.88° | 0.300 m | 5.07 % |
| browser #132 wasm, no wind | 678.31 | 10.598 s | 8.098 s | 19.58° | 0.299 m | 4.77 % |
| browser JS, no wind | 694.60 | 9.994 s | 8.094 s | 19.31° | 0.299 m | 4.76 % |

On #133 the browser is 0.08 points below native at the same completion time. With ground effect and voltage sag now in the loop, MuJoCo 4.0.1 vs 3.17 differences show up, so this is not a bit-exact match. Wind costs much more on #133: about 4.4 s longer and 143 points lower than its no-wind run, because the hop now fights the gusts with sagging thrust and ram drag.

**Verified** in headless Chrome with the #133 pkg. The page picks wasm by default on desktop and phone.
- **Desktop:** Space climbs from the 0.6 m hold to 1.4 m, D strafes 0.87 m right, A strafes back, Shift descends 0.76 m, then it lands and stows.
- **Phone (390×844):** touch Climb reaches 1.6 m, joystick right strafes 0.8 m, Descend drops 0.88 m. All 8 touch buttons are at least 48 px and the panel clears them.
- **Other builds:** with #132 or main's pkg, the default is "JS controller".
- No console errors.

**Next:** add the `assets/tbd` ↔ `web/tbd/model` match check.
