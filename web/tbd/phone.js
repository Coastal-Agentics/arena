// TBD sim: phone helpers (layout, DPR, wasm download progress, persisted UI state).
// Pure functions where possible so tests/web/tbd.test.mjs can cover them without a browser.

// Pinned MuJoCo 4.0.1 wasm: 11,201,302 bytes decoded. The CDN usually serves it brotli-compressed, so
// content-length is the compressed size and cannot be compared with the decoded bytes we read.
export const MUJOCO_WASM_URL = 'https://cdn.jsdelivr.net/npm/@mujoco/mujoco@4.0.1/mujoco.wasm';
export const MUJOCO_WASM_BYTES = 11201302;
export const HUD_KEY = 'arena.tbd.hudCollapsed';
export const ROTATE_KEY = 'arena.tbd.rotateHintDismissed';

// Phone-like: a coarse pointer, or a small screen (short side under 600 CSS px).
export function isMobileLike({ coarse = false, width = 1280, height = 800 } = {}) {
  return !!coarse || Math.min(width, height) < 600;
}

// Device pixel ratio cap: about 1.5 on phones, 2 on desktop.
export function pickDpr({ devicePixelRatio = 1, mobile = false } = {}) {
  const dpr = Number.isFinite(devicePixelRatio) && devicePixelRatio > 0 ? devicePixelRatio : 1;
  return Math.min(dpr, mobile ? 1.5 : 2);
}

// Render quality from the device class. lite = Lambert instead of PBR materials and no anisotropic
// filtering (both much cheaper per pixel on phones).
export function renderQuality(mobile) {
  return mobile ? { shadows: false, shadowMap: 0, antialias: false, lite: true } : { shadows: true, shadowMap: 2048, antialias: true, lite: false };
}

export function layoutFor({ width, height, mobile = false }) {
  const orientation = width >= height ? 'landscape' : 'portrait';
  return { orientation, compact: mobile || Math.min(width, height) < 600, rotateHint: mobile && orientation === 'portrait' };
}

// Vertical view shift (px) that centres the vehicle in the free band between the top UI (HUD/bar)
// and the bottom UI (thumb controls). Feed it to camera.setViewOffset(w, h, 0, shift, w, h).
export function viewShift(height, topPx, bottomPx) {
  const top = Math.max(0, topPx || 0), bottom = Math.max(0, bottomPx || 0);
  if (top + bottom >= height) return 0;
  return Math.round((bottom - top) / 2);
}

// Download size to show progress against. A content-length is only usable when the body is not
// content-encoded (otherwise it is the compressed size); else use the known/approximate size.
export function progressTotal({ contentLength = null, contentEncoding = null, approx = MUJOCO_WASM_BYTES } = {}) {
  const n = Number.parseInt(contentLength ?? '', 10);
  const encoded = contentEncoding && !/^\s*identity\s*$/i.test(contentEncoding);
  if (Number.isFinite(n) && n > 0 && !encoded) return { total: n, exact: true };
  return { total: approx, exact: false };
}

// Fraction in [0, 1). Never reports done before the bytes are all in (the caller sets 1 at the end).
export function progressFraction(loaded, total) {
  if (!(total > 0)) return 0;
  return Math.max(0, Math.min(0.99, loaded / total));
}

export const formatMB = bytes => `${(bytes / 1e6).toFixed(1)} MB`;

// Stream a fetch with progress. onProgress({ loaded, total, exact, fraction }). Resolves to a Uint8Array.
export async function fetchWithProgress(url, onProgress = () => {}, { fetchImpl = globalThis.fetch, approx = MUJOCO_WASM_BYTES } = {}) {
  const res = await fetchImpl(url);
  if (!res.ok) throw new Error(`HTTP ${res.status} for ${url}`);
  const h = res.headers;
  let { total, exact } = progressTotal({ contentLength: h.get('content-length'), contentEncoding: h.get('content-encoding'), approx });
  if (!res.body || !res.body.getReader) {   // no streaming: one shot
    const buf = new Uint8Array(await res.arrayBuffer());
    onProgress({ loaded: buf.length, total: buf.length, exact: true, fraction: 1 });
    return buf;
  }
  const reader = res.body.getReader(), chunks = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value); loaded += value.length;
    if (loaded > total) { total = Math.round(loaded * 1.05); exact = false; }
    onProgress({ loaded, total, exact, fraction: progressFraction(loaded, total) });
  }
  const out = new Uint8Array(loaded);
  let o = 0; for (const c of chunks) { out.set(c, o); o += c.length; }
  onProgress({ loaded, total: loaded, exact: true, fraction: 1 });
  return out;
}

// Persisted booleans (localStorage may be missing or throw in private modes).
export function loadFlag(storage, key) {
  try { return storage?.getItem(key) === '1'; } catch { return false; }
}
export function saveFlag(storage, key, value) {
  try { storage?.setItem(key, value ? '1' : '0'); } catch { /* ignore */ }
}

// Rolling frame-rate meter. tick(nowMs) per rendered frame; fps over the last window.
export function createFpsMeter(windowMs = 1000) {
  const times = [];
  return {
    tick(now) { times.push(now); while (times.length > 2 && now - times[0] > windowMs) times.shift(); },
    get fps() { if (times.length < 2) return 0; return (times.length - 1) * 1000 / (times[times.length - 1] - times[0]); },
  };
}
