/* Coastal Agentics: a quiet flock in the background.
 * A small grey swarm (boids: separation, alignment, cohesion) that hovers,
 * bounces softly off the viewport edges, and drifts into frame as you scroll.
 * Vanilla JS, no dependencies, no build step. Remove the <script> tag to turn it off.
 */
(function () {
  'use strict';

  /* ---- Tuning knobs ------------------------------------------------------ */
  var CONFIG = {
    count: 56,              // dots on wide screens
    countSmall: 28,         // dots on narrow screens
    smallBreakpoint: 640,   // px; below this width, use countSmall
    reducedCount: 18,       // static dots shown when prefers-reduced-motion is set (0 = none)
    color: '100,110,125',   // RGB of the dots
    opacityMin: 0.18,       // dot opacity range: keep low so text stays fully legible
    opacityMax: 0.28,
    sizeMin: 1.5,           // dot radius range, CSS px
    sizeMax: 2.5,
    maxSpeed: 32,           // CSS px per second
    minSpeed: 9,            // keeps the flock from stalling
    maxForce: 22,           // steering limit, px/s^2
    perception: 90,         // px; how far a dot "sees" neighbours
    separationDist: 34,     // px; personal space
    separation: 1.5,        // boids weights
    alignment: 0.55,
    cohesion: 0.22,
    wander: 0.3,            // pull toward the slowly wandering guide point once in frame (0 = free flock)
    jitter: 6,              // random wander, px/s^2
    hover: 1.6,             // px of gentle bobbing around each dot's path
    edgeMargin: 70,         // px; soft turn zone near the walls
    scrollNudge: 0.1,       // velocity added per px scrolled (px/s per px), in the scroll direction
    scrollNudgeMax: 3,      // cap on the per-event nudge, px/s
    entryScroll: 0.6,       // fraction of a viewport height of scrolling for the flock to fully enter
    entryEase: 1.4,         // seconds; how slowly the entry follows the scroll
    maxDpr: 2,              // canvas resolution cap
    fps: 60                 // frame cap
  };
  /* ------------------------------------------------------------------------ */

  if (!window.HTMLCanvasElement || !document.body) return;

  var canvas = document.createElement('canvas');
  canvas.setAttribute('aria-hidden', 'true');
  canvas.className = 'flock';
  var s = canvas.style;
  s.position = 'fixed'; s.left = '0'; s.top = '0';
  s.width = '100%'; s.height = '100%';
  s.zIndex = '-1'; s.pointerEvents = 'none'; s.display = 'block';
  document.body.insertBefore(canvas, document.body.firstChild);
  var ctx = canvas.getContext('2d');
  if (!ctx) return;

  var reduceQuery = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  var W = 0, H = 0, dpr = 1;
  var dots = [];
  var raf = 0, last = 0, clock = 0;
  var entry = 0, entryTarget = 0;   // 0 = flock waits off to the right, 1 = fully in frame
  var lastScroll = window.pageYOffset || 0;
  var frameMs = 1000 / CONFIG.fps;

  function rand(a, b) { return a + Math.random() * (b - a); }
  function reduced() { return !!(reduceQuery && reduceQuery.matches); }
  function targetCount() { return W < CONFIG.smallBreakpoint ? CONFIG.countSmall : CONFIG.count; }

  function makeDot(onScreen) {
    var a = rand(0, Math.PI * 2), sp = rand(CONFIG.minSpeed, CONFIG.maxSpeed * 0.6);
    return {
      // Start mostly just past the right edge, so the flock drifts in as you scroll.
      x: onScreen ? rand(0, W) : rand(W * 1.0, W * 1.3),
      y: onScreen ? rand(0, H) : rand(H * 0.35, H * 0.8),
      vx: Math.cos(a) * sp, vy: Math.sin(a) * sp,
      r: rand(CONFIG.sizeMin, CONFIG.sizeMax),
      fill: 'rgba(' + CONFIG.color + ',' + rand(CONFIG.opacityMin, CONFIG.opacityMax).toFixed(3) + ')',
      phase: rand(0, Math.PI * 2), freq: rand(0.4, 0.9)
    };
  }

  function sizeCanvas() {
    var oldW = W, oldH = H;
    W = window.innerWidth; H = window.innerHeight;
    dpr = Math.min(window.devicePixelRatio || 1, CONFIG.maxDpr);
    canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (oldW && oldH) {            // keep dots where they were, proportionally
      for (var i = 0; i < dots.length; i++) { dots[i].x *= W / oldW; dots[i].y *= H / oldH; }
    }
  }

  function fillTo(n, onScreen) {
    while (dots.length < n) dots.push(makeDot(onScreen));
    if (dots.length > n) dots.length = n;
  }

  function updateEntryTarget() {
    var y = window.pageYOffset || 0;
    var scrollable = document.documentElement.scrollHeight - H > 40;
    var p = scrollable ? Math.min(1, y / (H * CONFIG.entryScroll)) : 1;
    if (p > entryTarget) entryTarget = p;   // once in, the flock stays
  }

  function limit(x, y, m) {
    var l = Math.sqrt(x * x + y * y);
    return l > m ? [x / l * m, y / l * m] : [x, y];
  }

  function step(dt) {
    clock += dt;
    entry += (entryTarget - entry) * Math.min(1, dt / CONFIG.entryEase);
    var e = entry * entry * (3 - 2 * entry);           // smoothstep
    var inFrame = e > 0.98;                            // right wall only applies once the flock is in
    // A soft guide point: first a waiting spot just off the right edge, then
    // (as the flock enters) a slow Lissajous wander around the viewport.
    var wx = W * (0.5 + 0.3 * Math.sin(clock * 0.07 + 1.2)), wy = H * (0.5 + 0.25 * Math.sin(clock * 0.05));
    var holdX = W * 1.12 + (wx - W * 1.12) * e;
    var holdY = H * 0.55 + (wy - H * 0.55) * e;
    var hold = 1 - (1 - CONFIG.wander) * e, x75 = W * 0.75;
    // Move the guide on the flock as a whole (via its centre) so it drifts without bunching up.
    var mx = 0, my = 0;
    for (var c = 0; c < dots.length; c++) { mx += dots[c].x; my += dots[c].y; }
    if (dots.length) { mx /= dots.length; my /= dots.length; }
    var guide = limit((holdX - mx) * 0.05, (holdY - my) * 0.05, CONFIG.maxForce * 0.8);
    var P2 = CONFIG.perception * CONFIG.perception, S2 = CONFIG.separationDist * CONFIG.separationDist;
    var n = dots.length, i, j, d, o;

    for (i = 0; i < n; i++) {
      d = dots[i];
      var sx = 0, sy = 0, ax = 0, ay = 0, cx = 0, cy = 0, k = 0;
      for (j = 0; j < n; j++) {
        if (j === i) continue;
        o = dots[j];
        var dx = d.x - o.x, dy = d.y - o.y, q = dx * dx + dy * dy;
        if (q > P2 || q === 0) continue;
        k++; ax += o.vx; ay += o.vy; cx += o.x; cy += o.y;
        if (q < S2) { sx += dx / q; sy += dy / q; }
      }
      var fx = 0, fy = 0, st;
      if (k) {
        st = limit(ax / k - d.vx, ay / k - d.vy, CONFIG.maxForce);
        fx += st[0] * CONFIG.alignment; fy += st[1] * CONFIG.alignment;
        st = limit(cx / k - d.x, cy / k - d.y, CONFIG.maxForce);
        fx += st[0] * CONFIG.cohesion; fy += st[1] * CONFIG.cohesion;
        st = limit(sx * 400, sy * 400, CONFIG.maxForce);
        fx += st[0] * CONFIG.separation; fy += st[1] * CONFIG.separation;
      }
      // hover and jitter
      fx += (Math.random() - 0.5) * 2 * CONFIG.jitter;
      fy += (Math.random() - 0.5) * 2 * CONFIG.jitter + Math.sin(clock * d.freq + d.phase) * CONFIG.jitter * 0.5;
      // guide: holds the flock off-screen until you scroll, then a gentle wander
      fx += guide[0] * hold; fy += guide[1] * hold;
      // while entering, dots still near or past the right edge get a firm push left
      var late = e < 0.98 ? e * Math.max(0, Math.min(1, (d.x - x75) / (W * 0.25))) : 0;
      fx -= CONFIG.maxForce * 2 * late;
      // soft walls
      var m = CONFIG.edgeMargin, push = CONFIG.maxForce * 1.5;
      if (d.x < m) fx += push * (1 - d.x / m);
      if (inFrame && d.x > W - m) fx -= push * Math.min(2, (d.x - (W - m)) / m);
      if (d.y < m) fy += push * (1 - d.y / m);
      if (d.y > H - m) fy -= push * (1 - (H - d.y) / m);

      d.vx += fx * dt; d.vy += fy * dt;
      // dots still off to the right may hurry a little while entering
      var cap = CONFIG.maxSpeed * (1 + 2.5 * late);
      var sp = Math.sqrt(d.vx * d.vx + d.vy * d.vy);
      if (sp > cap) { d.vx *= cap / sp; d.vy *= cap / sp; }
      else if (sp < CONFIG.minSpeed && sp > 0) { d.vx *= CONFIG.minSpeed / sp; d.vy *= CONFIG.minSpeed / sp; }
    }

    for (i = 0; i < n; i++) {
      d = dots[i];
      d.x += d.vx * dt; d.y += d.vy * dt;
      // bounce off the walls (the right wall only once the flock has entered)
      if (d.x < 0) { d.x = 0; d.vx = Math.abs(d.vx) * 0.8; }
      if (d.y < 0) { d.y = 0; d.vy = Math.abs(d.vy) * 0.8; }
      if (d.y > H) { d.y = H; d.vy = -Math.abs(d.vy) * 0.8; }
      if (inFrame && d.x > W) { d.x = W; d.vx = -Math.abs(d.vx) * 0.8; }
      else if (d.x > W * 1.5) { d.x = W * 1.5; d.vx = -Math.abs(d.vx) * 0.8; }
    }
  }

  function draw(hover) {
    ctx.clearRect(0, 0, W, H);
    for (var i = 0; i < dots.length; i++) {
      var d = dots[i];
      var hx = hover ? Math.cos(clock * d.freq * 0.7 + d.phase) * CONFIG.hover : 0;
      var hy = hover ? Math.sin(clock * d.freq + d.phase) * CONFIG.hover : 0;
      var x = d.x + hx, y = d.y + hy;
      if (x < -4 || x > W + 4 || y < -4 || y > H + 4) continue;
      ctx.fillStyle = d.fill;
      ctx.beginPath(); ctx.arc(x, y, d.r, 0, Math.PI * 2); ctx.fill();
    }
  }

  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (now - last < frameMs - 1) return;        // 60fps cap on faster displays
    var dt = Math.min(0.05, (now - last) / 1000); // clamp after stalls
    last = now;
    step(dt);
    draw(true);
  }

  function start() {
    if (raf || document.hidden || reduced()) return;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  }
  function stop() { if (raf) cancelAnimationFrame(raf); raf = 0; }

  function showStatic() {     // reduced motion: a sparse, still dot field
    stop();
    dots = [];
    fillTo(CONFIG.reducedCount, true);
    draw(false);
  }

  function init() {
    sizeCanvas();
    if (reduced()) { showStatic(); return; }
    dots = [];
    fillTo(targetCount(), false);
    updateEntryTarget();
    entry = entryTarget;      // if the page loads already scrolled, start in frame
    start();
  }

  var resizeTimer = 0;
  window.addEventListener('resize', function () {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(function () {
      sizeCanvas();
      if (reduced()) { showStatic(); return; }
      fillTo(targetCount(), entry > 0.5);
      updateEntryTarget();
      if (!raf) draw(true);
    }, 150);
  });

  window.addEventListener('scroll', function () {
    var y = window.pageYOffset || 0, dy = y - lastScroll;
    lastScroll = y;
    if (reduced()) return;
    updateEntryTarget();
    var nudge = Math.max(-CONFIG.scrollNudgeMax, Math.min(CONFIG.scrollNudgeMax, dy * CONFIG.scrollNudge));
    for (var i = 0; i < dots.length; i++) dots[i].vy += nudge * (0.6 + 0.4 * Math.random());
  }, { passive: true });

  document.addEventListener('visibilitychange', function () { document.hidden ? stop() : start(); });

  if (reduceQuery) {
    var onChange = function () { stop(); init(); };
    if (reduceQuery.addEventListener) reduceQuery.addEventListener('change', onChange);
    else if (reduceQuery.addListener) reduceQuery.addListener(onChange);
  }

  init();
})();
