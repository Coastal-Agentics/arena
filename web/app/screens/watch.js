// Watch (#/watch/<game>?slots=…&seed=N): the match plays inside the app with a live board, then
// a finish card: who won, where your Nyborg placed, and what next (again, new seed, tweak, share).
// The game module runs the engine; the screen never names a game.
import { el, face } from "../ui.js";
import { arenas, matchFor, viewerLink } from "../../games/index.js";
import { isHouse } from "../house.js";
import { href } from "../router.js";
import { resolveMatch, freshSeed, matchQuery } from "../match.js";
import { mountPlayer } from "../player.js";

const ordinal = (n) => {
  const s = ["th", "st", "nd", "rd"], v = n % 100;
  return `${n}${s[(v - 20) % 10] || s[v] || s[0]}`;
};

export default {
  title: (args, app) => `Watch: ${arenas(app.engine).find((m) => m.id === args.game)?.title || "match"}`,
  mount(root, { args, params, app }) {
    const { engine, lib } = app;
    const mod = arenas(engine).find((m) => m.id === args.game);
    if (!mod) { app.go(href([]), { replace: true }); return; }
    const resolved = resolveMatch(app, mod, params);
    const { ids, who } = resolved;
    let seed = resolved.seed;
    const mine = ids.filter((id) => !isHouse(id));
    const fav = mine.includes(lib.selected) ? lib.selected : mine[0] || null;

    let session = null, player = null, finished = false;
    const title = el("h1", { tabindex: "-1" });
    const stage = el("div", { class: "match-stage" });
    const side = el("aside", { class: "match-side", "aria-live": "polite" });

    const tag = (id) => el("span", { class: `tag ${isHouse(id) ? "house" : "yours"}`, text: isHouse(id) ? "House" : "Yours" });
    const shareUrl = () => new URL(viewerLink(engine, mod, matchFor(engine, mod, ids.map(who), seed)), location.href).href;

    function run() {
      player?.stop();
      session?.free();
      finished = false;
      history.replaceState(null, "", href(["watch", mod.id], matchQuery(ids, seed)));
      title.textContent = `${mod.title} · seed ${seed}`;
      session = mod.start(engine, { ...matchFor(engine, mod, ids.map(who), seed), you: ids.map((id, i) => (isHouse(id) ? -1 : i)).filter((i) => i >= 0) });
      player = mountPlayer(stage, session, {
        label: `${mod.title}: ${ids.map((id) => who(id).name).join(", ")}`,
        speed: player?.speed || 1, onTick: renderBoard, onFinish: renderFinish, onRestart: run,
      });
      renderBoard(session);
      window.__watch = { get session() { return session; }, get finished() { return finished; }, ids, get seed() { return seed; }, shareUrl };
    }

    function renderBoard(s) {
      if (finished) return;
      const rows = s.board();
      side.replaceChildren(
        el("h2", { text: rows.some((r) => r.place) ? "Standings" : "Health" }),
        el("ol", { class: "board" }, rows.map((r) => el("li", { class: isHouse(ids[r.slot]) ? "" : "mine" },
          el("span", { class: "swatch", style: `background:${r.hair}` }),
          r.place ? el("strong", { class: "place", text: `${r.place}.` }) : null,
          el("span", { class: "nm", text: r.name }),
          el("span", { class: "muted small note", text: r.note }),
          r.hp != null ? el("span", { class: "hp" }, el("span", { style: `width:${Math.round(r.hp * 100)}%` })) : null))),
        el("p", { class: "muted small", text: mod.blurb }));
    }

    function renderFinish(res) {
      finished = true;
      const favPlace = fav ? res.place(ids.indexOf(fav)) : null;
      const favName = fav ? who(fav).name : "";
      const won = res.winner != null && !isHouse(ids[res.winner]);
      const headline = res.winner == null ? res.headline : won ? `🏆 ${res.headline}` : `${who(ids[res.winner]).name} wins this one.`;
      const lede = !fav ? "The house Nyborgs had the arena to themselves."
        : res.winner == null ? `${favName} held on for a draw.`
        : favPlace === 1 ? `${favName} took it in ${(res.time(ids.indexOf(fav)) ?? res.seconds).toFixed(1)}s. Nicely built.`
        : `${favName} finished ${ordinal(favPlace)} of ${ids.length}. Tweak the build and go again?`;
      const copyBtn = el("button", { type: "button", class: "btn", id: "share", text: "Copy share link" });
      copyBtn.addEventListener("click", async () => {
        const url = shareUrl();
        try { await navigator.clipboard.writeText(url); copyBtn.textContent = "Copied!"; }
        catch { copyBtn.replaceWith(el("input", { class: "share-url", value: url, readonly: true, onfocus: (e) => e.target.select() })); }
      });
      side.replaceChildren(el("section", { class: "finish-card", "aria-labelledby": "finish-h" },
        el("p", { class: "eyebrow", text: "Finish" }),
        el("h2", { id: "finish-h", text: headline }),
        el("p", { class: "lede", text: lede }),
        el("ol", { class: "podium" }, res.order.map((slot) => el("li", { class: slot === res.winner ? "winner" : "" },
          face(who(ids[slot]), { scale: 0.7, y: 0.7 }, { class: "podium-face" }),
          el("span", { class: "place", text: res.place(slot) ? ordinal(res.place(slot)) : "–" }),
          el("span", { class: "nm", text: who(ids[slot]).name }), tag(ids[slot]),
          el("span", { class: "muted small", text: res.detail(slot) })))),
        el("div", { class: "row gap finish-acts" },
          el("button", { type: "button", class: "btn primary", id: "again", text: "Again ▶", title: "Same seed, same match", onclick: run }),
          el("button", { type: "button", class: "btn", id: "new-seed", text: "🎲 New seed", onclick: () => { seed = freshSeed(); run(); } }),
          fav ? el("a", { class: "btn", id: "tweak", href: app.tweakHref(fav), text: `Tweak ${favName}`, onclick: () => app.safe(() => lib.select(fav)) }) : null,
          el("a", { class: "btn", href: href(["play", mod.id], matchQuery(ids, seed)), text: "Change lineup" }),
          copyBtn),
        el("p", { class: "muted small", text: "The share link opens this exact match for anyone, looks and names included." })));
      side.querySelector("h2")?.focus?.();
    }

    root.replaceChildren(
      el("div", { class: "intro" },
        el("p", { class: "eyebrow" }, el("a", { href: href(["play", mod.id], matchQuery(ids, seed)), text: "← Lineup" }), " · ", el("a", { href: href([]), text: "Your Nyborgs" })),
        title),
      el("div", { class: "watch" }, stage, side));
    run();
    return () => { player?.stop(); session?.free(); session = null; };
  },
};
