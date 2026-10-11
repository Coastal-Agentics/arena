// Small DOM helpers for the Nyborg app: el(), one animation loop for every Nyborg preview, and
// the library opener (localStorage when allowed, else this session only). No framework.
import { drawNyborg, idleBob } from "../nyborg-draw.js";
import { Library, MemoryStorage } from "../nyborg-library.js";

/** el("button", {class, text, onclick, ...attrs}, ...children); null/false children are skipped. */
export function el(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") e.className = v;
    else if (k === "text") e.textContent = v;
    else if (k.startsWith("on") && typeof v === "function") e.addEventListener(k.slice(2), v);
    else if (v !== false && v != null) e.setAttribute(k, v === true ? "" : v);
  }
  for (const k of kids.flat()) if (k != null && k !== false) e.append(k);
  return e;
}

/** node.replaceChildren(...kids) without the null/false ones (replaceChildren would print "null"). */
export const fill = (node, ...kids) => node.replaceChildren(...kids.flat().filter((k) => k != null && k !== false));

export const titleCase = (s) => s.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

// --- previews ---------------------------------------------------------------------------------
const previews = new Set(); // {canvas, nyborg(), scale, flip, y}
let looping = false;

/**
 * Draw `nyborg()` ({id, look}) into `canvas` every frame, bobbing. `y` is where the head sits
 * (0..1 of the height). `flip` turns it around every 2.4 s to show the mirror the viewer does.
 * `fit` (px of canvas width per unit of scale) sizes big faces to their canvas instead of `scale`.
 */
export function addPreview(canvas, nyborg, { scale = 1, flip = false, y = 0.7, fit = 0 } = {}) {
  previews.add({ canvas, nyborg, scale, flip, y, fit });
  if (!looping) { looping = true; requestAnimationFrame(loop); }
  return canvas;
}

function paint(p, t) {
  const c = p.canvas;
  if (!c.isConnected) { previews.delete(p); return; }
  const dpr = window.devicePixelRatio || 1;
  const w = c.clientWidth || c.width, h = c.clientHeight || c.height;
  if (c.width !== Math.round(w * dpr)) { c.width = Math.round(w * dpr); c.height = Math.round(h * dpr); }
  const ctx = c.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  const n = p.nyborg();
  if (!n?.look) return;
  const facing = p.flip && Math.floor(t / 2400) % 2 === 1 ? -1 : 1;
  const scale = p.fit ? w / p.fit : p.scale;
  ctx.save();
  ctx.translate(w / 2, h * p.y);
  drawNyborg(ctx, {
    hair: n.look.hair_color, style: n.look.hair_style, id: n.id || n.name || "nyborg",
    strands: n.look.strands, accessories: n.look.accessories,
    facing, bob: idleBob((t / 1000) * 60) * scale, scale,
  });
  ctx.restore();
}

function loop(t) {
  for (const p of previews) paint(p, t);
  if (previews.size) requestAnimationFrame(loop); else looping = false;
}

/** A canvas showing one Nyborg (sized by CSS); see addPreview. */
export function face(nyborg, opts = {}, attrs = {}) {
  const c = el("canvas", { width: 96, height: 96, "aria-hidden": "true", ...attrs });
  return addPreview(c, typeof nyborg === "function" ? nyborg : () => nyborg, opts);
}

// --- storage ----------------------------------------------------------------------------------
/** The Nyborg library over localStorage when the browser allows it; else memory (this tab only). */
export function openLibrary(win = window) {
  let storage, blocked = false;
  try {
    const probe = "arena.nyborg.__probe";
    win.localStorage.setItem(probe, "1");
    win.localStorage.removeItem(probe);
    storage = win.localStorage;
  } catch {
    storage = new MemoryStorage();
    blocked = true;
  }
  const lib = new Library(storage);
  lib.load();
  return { lib, storage, blocked };
}
