// The "you" marker over your Nyborg in the app's watch canvas: a small pill with a pointer.
// App only (the viewer doesn't know whose Nyborg is whose). Cosmetic, never reaches the engine.
export function drawYouMarker(ctx, x, y, { alpha = 1, label = "you" } = {}) {
  ctx.save();
  ctx.globalAlpha = alpha;
  ctx.font = "bold 11px system-ui, sans-serif";
  const w = ctx.measureText(label).width + 12, h = 16;
  ctx.fillStyle = "#FFD45C";
  ctx.strokeStyle = "#1F3147";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.roundRect(x - w / 2, y - h - 5, w, h, 8);
  ctx.moveTo(x - 4, y - 5.5); ctx.lineTo(x, y); ctx.lineTo(x + 4, y - 5.5);
  ctx.fill();
  ctx.stroke();
  ctx.fillStyle = "#1F3147";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(label, x, y - h / 2 - 5);
  ctx.restore();
}
