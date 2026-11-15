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

## Follow-up (Nov 14, 3 PM): phone pass

**Brief:** make `/arena/tbd/` comfortable on a phone in both orientations. The page should fit the screen with no page scroll and no zoom, and have big thumb controls that keep clear of the vehicle. The brief also asked for a HUD that collapses and remembers its state, a progress bar for the 11 MB MuJoCo wasm, a render budget aiming for 30+ fps with 4× CPU throttle, and a rotate hint you can dismiss. The wasm controller stays the default and `?ctrl=js` still works.

**Steps**
1. **Fit:** the viewport meta now has `viewport-fit=cover` and `user-scalable=no`.
   - `html` and `body` are `100dvh`, `position: fixed`, `overscroll-behavior: none` and `touch-action: none`. Safe-area insets go into CSS variables and pad the bar, HUD and controls.
   - Zoom is blocked four ways: iOS `gesturestart`/`gesturechange`, `dblclick`, a `touchmove` guard (multi-touch, or anything outside the help panel) and a double-tap guard on `touchend` that leaves buttons and links alone.
2. **Thumb controls:** the left virtual stick drives forward/back and turns; in flight its X axis strafes. The right pad grid has Q/E turn, Climb, a wide Fold/fly button and Descend.
   - All of it is pointer events with per-pointer capture, so the stick and a pad can be held together.
   - In landscape the buttons shrink to 58 px and the stick to 128 px. Hop, Cam and Reset move into the top bar on phones, and the help panel and footer are hidden.
3. **View offset:** `layout()` measures the free band between the top UI (the HUD in portrait, the bar in landscape) and the controls. It then shifts the camera with `setViewOffset`, so the vehicle sits in that band instead of under a thumb. It re-runs on resize, rotation and HUD collapse.
4. **HUD:** the ▾ toggle collapses the HUD to one line (mode · AGL · battery). The state is saved in `localStorage` (`arena.tbd.hudCollapsed`) and the save is wrapped in a try/catch for private modes. In landscape the HUD drops the thrust bars and the clock so it stays above the stick.
5. **Wasm progress:** `fetchWithProgress` streams the wasm through a reader and hands the bytes to `loadMujoco({ wasmBinary })`.
   - The CDN serves it brotli (content-length 2,696,116 against 11,201,302 decoded). `progressTotal` therefore trusts content-length only when the body isn't encoded; otherwise it uses the known size and grows it if the download runs past.
   - The bar never shows 100 % before the last byte. three.js and the MuJoCo JS load in parallel with the download.
6. **Render budget** (phones; `?quality=high` opts out):
   - DPR is capped at 1.5 on phones and 2 on desktop. Phones get no shadows, no MSAA, Lambert instead of PBR materials, no anisotropic filtering on the ground and 16-segment round geometry.
   - The HUD DOM updates at 10 Hz, camera vectors are reused and the eye material only changes when the mode does. `TBD.perf()` reports fps, real-time factor, DPR, shadows and orientation.
7. **Rotate hint:** portrait phones get a "Rotate your phone for a wider view" pill. Its × dismisses it for good (`arena.tbd.rotateHintDismissed`).
8. **Tests:** the new `tests/web/tbd-phone.test.mjs` adds 13 tests. They cover the mobile, DPR, quality, layout and view-shift helpers; progress totals for encoded vs plain bodies; `fetchWithProgress` with a fake streaming fetch (exact length, encoded with a growing total, no-stream fallback, HTTP error); flag storage with a throwing store; the fps meter; and the HTML/CSS hooks. The suite total is 100/100.

**What failed along the way**
- The first landscape layout put the HUD bottom (about 270 px) below the stick top (248 px). Dropping the thrust bars and clock from the landscape HUD fixed it.
- Landscape fps with 4× throttle was 25.7 on the first run. Lambert materials brought it to 29.7, still short of 30. Turning off 8× anisotropic filtering on the tiled ground was the big win: it went to 53.5.
- The double-tap guard first swallowed quick button taps, so it now skips `button` and `a`.
- `layout()` could run before it was defined when the touch UI showed early, so it now waits for a ready flag.

**Results:** headless Chrome with mobile emulation (device scale 3, touch). This is SwiftShader software GL on the box, so absolute numbers are pessimistic for a real phone GPU. Each fps figure is the mean over 4 s of flight.

| run | DPR | shadows | fps | fps, 4× CPU throttle |
|---|---|---|---|---|
| phone portrait 390×844 (wasm) | 1.5 | off | 60.0 | 59.5 |
| phone landscape 844×390 (wasm) | 1.5 | off | 52.5 | 53.5 |
| phone portrait, `?ctrl=js` | 1.5 | off | 60.0 | 60.0 |
| phone portrait, `?quality=high` | 1.5 | on | 19.7 | 15.0 |
| desktop 1280×800 | 1 | on | 16.1 | 14.4 |

The real-time factor stayed about 1.00 in every run.

**Verified:**
- `scrollHeight` equals `innerHeight` in both orientations (844/844 and 390/390).
- Every control is inside the viewport, and the vehicle's screen point is clear of the HUD, stick and pads.
- A pinch gesture leaves `visualViewport.scale` at 1.
- Stick plus Climb held together strafes and climbs at once.
- The HUD collapse survives a reload.
- The progress bar shows during a throttled load ("1.6 MB of about 11.2 MB").
- The rotate hint shows in portrait only and its dismissal sticks.
- wasm is the default and `?ctrl=js` works.
- No console errors.

**Not tested:** real devices. That covers iOS Safari's gesture and toolbar behaviour, Android Chrome, real multi-touch hardware, notch safe-area insets and real-GPU frame rates.

**Next:** try it on a real iPhone and Android phone, then the `assets/tbd` ↔ `web/tbd/model` match check.
