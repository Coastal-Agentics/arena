# Architecture: a Rust-driven, web-playable TBD sim

_Internal recommendation, Nov 14, 2026. Physics sim with estimated parameters._

## What exists
- **Coastal-Agentics/arena** (checked at `a0a6ce3`, Nov 8):
  - A Cargo workspace: `engine` (deterministic 2D core), `engine-cli`, `engine-wasm`, and `games/{tank,racing,catalog}`. `engine-py` is a separate pyo3 workspace.
  - `engine` has a generic `Rules` trait with a fixed tick, seeded ChaCha8 RNG, state hash and replays (ADR-014 B1).
  - `engine-wasm` is plain `wasm32-unknown-unknown` plus `wasm-bindgen` (pinned `=0.2.100`). Its glue is committed in `web/pkg/`, and it feeds the static viewer on Pages.
- **No MuJoCo or Rust–MuJoCo bindings exist in arena today.**
  - MuJoCo appears only in docs: CHARTER, ADR-010 ("Rust core is a candidate browser viewer for Saltmarsh… Alternative: a MuJoCo-native web viewer") and ADR-012 (Bevy as a candidate 3D renderer, not scheduled).
  - Saltmarsh is the Python MuJoCo library.
- **Upstream (checked today):**
  - Google DeepMind publishes official MuJoCo WASM bindings as npm `@mujoco/mujoco` **4.0.1**, the same version as the `mujoco` pip package on the box. It's an Emscripten build with Embind classes (`MjModel`, `MjData`, `mj_step`, named access) and an in-memory FS for XML and meshes. A single-threaded build needs no special headers. The `.wasm` is 11.2 MB, or 2.8 MB gzipped.
  - `mujoco-rs` 7.1.0 (davidhozic) is a maintained high-level Rust wrapper over the MuJoCo C library, with a native viewer and renderer. It's native FFI, with no wasm target.

## Options
| | (a) MuJoCo WASM in browser + JS/TS UI + Rust/wasm game logic | (b) `mujoco-rs` server-side, stream state to browser | (c) others |
|---|---|---|---|
| Physics | MuJoCo 4.0.1 on the player's CPU (about 45× real time on the box in Node, single thread) | MuJoCo native on a server | (c1) Rust + MuJoCo C in one `wasm32-unknown-emscripten` module. (c2) A Rust physics engine (Rapier/Bevy) |
| Hosting | **Static, so GitHub Pages works.** No server and no cost | Needs a persistent server or GPU box, plus WebSocket/WebRTC | (c1) static. (c2) static |
| Latency and feel | Local input, 500 Hz physics, no network | Network round-trip on every input, which isn't ideal for flying | local |
| Fits arena | `engine-wasm` stays `wasm32-unknown-unknown` + wasm-bindgen. A TBD game crate runs beside MuJoCo, talking through flat arrays | The Rust sim becomes a service, which is a new ops surface | (c1) fights the existing wasm-bindgen/unknown-unknown toolchain and the pinned glue. (c2) drops MuJoCo and breaks the link to Saltmarsh and to training |
| Same model as training | yes, the same `tbd.xml` as Python/Saltmarsh | yes | (c1) yes. (c2) **no** |
| Rust native path | `mujoco-rs` for headless tests, batch runs and CI, with the same `tbd.xml` and the same Rust game crate | primary | — |
| Effort | lowest. **The prototype already runs** | medium–high | high |

## Pick: (a) MuJoCo WASM in the browser, with Rust/wasm game logic from the arena engine
This is the only option that keeps arena's model intact: a static Pages site with zero servers, the existing `wasm32-unknown-unknown` + wasm-bindgen toolchain, and the deterministic Rust core owning the rules. It also puts the exact same `tbd.xml` in front of players that Python/Saltmarsh will train on. Physics runs locally at 500 Hz, so flying feels immediate. The official bindings move in lockstep with the pip package (both 4.0.1), so web and Python can't drift. MuJoCo stays a black-box physics step. The Rust crate owns everything Coastal-specific, behind the same `Rules`-style boundary ADR-014 describes: flight assist and controller, modes, missions, scoring, seeded wind or gust scenarios, and the replay log. Natively, the same Rust crate plus `mujoco-rs` gives headless runs and CI. Option (b) remains the right later add-on for heavy jobs (RL training, large batch evaluations) without changing the browser path. This also settles ADR-010: it takes its stated alternative ("a MuJoCo-native web viewer") for physics, and keeps the Rust core for game logic.

### Data flow per physics tick (2 ms)
```
keys/gamepad ─▶ JS input ─▶ Rust/wasm TbdGame.step(obs: Float64Array) ─▶ ctrl: Float64Array
                                   ▲                                         │
          MjData.sensordata/qpos ──┘                     MjData.ctrl ◀───────┘ ─▶ mj_step (MuJoCo WASM)
Three.js renders MjData.geom_xpos/xmat every animation frame; the HUD reads TbdGame state.
```
Both wasm modules share nothing but small typed arrays: about 20 doubles in and 9 out per tick. That overhead is negligible next to `mj_step`.

## Next steps to wire into arena (proposals only; no repo changes made)
1. **Add a `games/tbd` crate (pure Rust, no MuJoCo dependency).**
   - Port `tbd_control.py`/`.js`: the flight assist, mode machine (ground → unfolding → flight → landing → folding) and battery model.
   - Add a mission layer: stair-hop challenge, timer and score.
   - Use the engine's seeded RNG for wind gusts, so a run is reproducible from seed + input log.
   - Unit-test it natively against recorded MuJoCo traces.
2. **Add `WasmTbd` to `engine-wasm`.** It's a `#[wasm_bindgen]` wrapper with `new(seed, config_json)`, `step(obs) -> ctrl` and `state_json()`, exported like `WasmRace`. The web page swaps its JS controller for it. The JS stays as a reference implementation in tests.
3. **Add a native harness.** Behind a feature flag, an `engine-cli` (or a separate `tbd-native`) binary would use `mujoco-rs` to load `tbd.xml`, run `games/tbd` headless and write replays.
   - Parity can't be bit-exact: WASM and native MuJoCo builds may differ in the last bits. So use a tolerance-based trace check, not arena's exact state hash.
4. **Add `web/tbd/` to arena** (or a new `sim` page): this prototype, with vendored `@mujoco/mujoco` 4.0.1 and three r187.
   - Check how Pages serves `.wasm` (MIME type and gzip). Otherwise ship it pre-compressed or lazy-load it.
   - Credit MuJoCo (Apache-2.0) and three.js (MIT) in THIRD_PARTY_NOTICES.
5. **Saltmarsh:** a Gymnasium env around the same `tbd.xml`, for learned hop and landing policies later. The web game would run them through an exported policy (ONNX or plain weights in Rust).
6. **Gate:** taking this into arena or Saltmarsh is a repo change and ADR-010 decision, so Nye decides. Nothing was pushed or opened.
