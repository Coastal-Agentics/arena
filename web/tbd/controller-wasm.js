// Adapter for Shockwave's Rust/wasm controller step (not landed yet).
//
// TODO(wasm-step): when the Rust controller PR lands in Coastal-Agentics/arena, load its wasm-bindgen
// package here and return an object with the same shape as createJsController() in controller.js:
//   { name: 'wasm', mode, auto, reset(state, startMode), startAuto(state), press(action), step(state, input) -> cmd }
// Then add a parity test in tests/web/tbd.test.mjs that feeds the same state/input sequence to both
// controllers and compares cmd.drive, cmd.thrust, cmd.fold and cmd.mode within a small tolerance.
// The JS controller stays as the test reference and the runtime fallback.

export const WASM_CONTROLLER_AVAILABLE = false;

// Resolves to a controller, or null when no wasm controller is available (callers fall back to JS).
export async function loadWasmController() {
  return null;
}

// Picks the controller for the page. `prefer` is 'wasm' or 'js' (from ?ctrl=); always falls back to JS.
export async function pickController(prefer, createJs) {
  if (prefer === 'wasm') {
    try { const c = await loadWasmController(); if (c) return c; } catch (err) { console.warn('[tbd] wasm controller failed, using JS', err); }
  }
  return createJs();
}
