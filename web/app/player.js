// The match player: a canvas, Play/Pause, Restart and speed, driving a game session from a game
// module's start() (docs/design/nyborg-app.md §6). 60 ticks a second times the speed, catch-up
// capped so a background tab doesn't fast-forward. Calls onTick for the board and onFinish once.
import { el } from "./ui.js?v=20261024b";

const TICK_HZ = 60;

export function mountPlayer(root, session, { label = "Match", speed = 1, onTick = () => {}, onFinish = () => {}, onRestart = () => {} } = {}) {
  const canvas = el("canvas", { class: "match", width: session.width, height: session.height, role: "img", "aria-label": label });
  const ctx = canvas.getContext("2d");
  const clock = el("span", { class: "clock", "aria-live": "off" });
  let playing = true, carry = 0, last = null, done = false, raf = 0, frames = 0;

  const playBtn = el("button", { type: "button", class: "btn small", text: "Pause", onclick: () => setPlaying(!playing) });
  const speeds = [1, 2, 4].map((s) => el("button", { type: "button", class: `btn small speed${s === speed ? " on" : ""}`, "aria-pressed": String(s === speed), text: `${s}×`, onclick: () => setSpeed(s) }));
  function setPlaying(p) { playing = p; last = null; playBtn.textContent = p ? "Pause" : "Play"; }
  function setSpeed(s) { speed = s; speeds.forEach((b) => { const on = b.textContent === `${s}×`; b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on)); }); }

  root.replaceChildren(canvas, el("div", { class: "row gap controls" },
    playBtn, el("button", { type: "button", class: "btn small", text: "Restart", onclick: () => onRestart() }),
    el("span", { class: "speeds row" }, speeds), clock));

  function frame(now) {
    if (!canvas.isConnected) return;
    if (playing && !session.over()) {
      if (last !== null) {
        carry += Math.min((now - last) / 1000, 0.1) * TICK_HZ * speed;
        const n = Math.floor(carry);
        if (n > 0) { carry -= n; session.step(n); }
      }
      last = now;
    }
    session.draw(ctx);
    clock.textContent = session.clock();
    if (!done && (++frames % 6 === 0 || session.over())) onTick(session);
    if (!done && session.over()) { done = true; onFinish(session.result()); }
    raf = requestAnimationFrame(frame);
  }
  raf = requestAnimationFrame(frame);
  return {
    stop() { cancelAnimationFrame(raf); },
    get speed() { return speed; },
    setPlaying, setSpeed,
  };
}
