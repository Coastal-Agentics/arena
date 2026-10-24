// Watch: the match plays inside the app with a live board, then a finish card: who won, where
// your Nyborg placed, and what next (again, new seed, tweak, share).
// - #/watch/<game>?slots=<id>,…&seed=N: your lineup (your Nyborgs and house Nyborgs by id).
// - #/watch?<viewer query>: someone's match link (the exact query arena.html reads). Its
//   Nyborgs can be saved to yours: build (engine-checked), look and name.
// The game module runs the engine; the screen never names a game.
import { el, face, fill } from "../ui.js?v=20261024a2";
import { arenas, matchFor, gameForQuery } from "../../games/index.js?v=20261024a2";
import { createNyborg, DEFAULT_LOOK, touch } from "../../nyborg-library.js?v=20261024a2";
import { HAIR_PALETTE } from "../../nyborg-draw.js?v=20261024a2";
import { isHouse } from "../house.js?v=20261024a2";
import { href } from "../router.js?v=20261024a2";
import { resolveMatch, freshSeed, matchQuery, seedOk } from "../match.js?v=20261024a2";
import { mountPlayer } from "../player.js?v=20261024a2";

const ordinal = (n) => {
  const s = ["th", "st", "nd", "rd"], v = n % 100;
  return `${n}${s[(v - 20) % 10] || s[v] || s[0]}`;
};
// An unnamed Nyborg goes by its behavior, as the engine and the viewer call it ("Follower").
const behaviorName = (b) => { const id = b?.behavior?.id || "nyborg"; return id[0].toUpperCase() + id.slice(1); };
const TAGS = { yours: "Yours", house: "House", theirs: "From the link" };

/** Your lineup: entries {n (a Nyborg), kind}, plus how to rebuild the match for a seed. */
function lineupFromIds(app, mod, params) {
  const r = resolveMatch(app, mod, params);
  return {
    seed: r.seed, notes: [],
    entries: r.ids.map((id) => ({ n: r.who(id), kind: isHouse(id) ? "house" : "yours", id })),
    match: (seed) => matchFor(app.engine, mod, r.ids.map(r.who), seed),
    hash: (seed) => href(["watch", mod.id], matchQuery(r.ids, seed)),
    lineupHref: (seed) => href(["play", mod.id], matchQuery(r.ids, seed)),
  };
}

/** Someone's match link: entries stand in for their Nyborgs (looks and names from the link). */
function lineupFromLink(app, mod, params) {
  const parsed = mod.parseQuery(app.engine, params);
  if (!seedOk(parsed.seed)) throw new Error(`The seed in this link (${parsed.seed}) isn't a whole number, so there's no match to play.`);
  const entries = parsed.slots.map((s, i) => ({
    kind: "theirs", slot: s, fellBack: parsed.fellBack[i],
    n: { id: `link-${i}`, name: s.name || behaviorName(s.build), look: s.look || { ...DEFAULT_LOOK, hair_color: HAIR_PALETTE[(i * 3) % HAIR_PALETTE.length].hex }, builds: { [mod.id]: s.build } },
  }));
  const match = (seed) => ({ game: mod.id, seed, slots: parsed.slots });
  return {
    seed: parsed.seed, notes: parsed.notes, entries, match,
    hash: (seed) => `#/watch?${mod.query(app.engine, match(seed))}`,
    lineupHref: null,
  };
}

export default {
  title: (args, app) => `Watch: ${arenas(app.engine).find((m) => m.id === args.game)?.title || "a match"}`,
  mount(root, { args, params, app }) {
    const { engine, lib } = app;
    const fromLink = !args.game;
    const mod = fromLink ? gameForQuery(engine, params) : arenas(engine).find((m) => m.id === args.game);
    if (!mod) { app.go(href([]), { replace: true }); return; }
    let lineup;
    try { lineup = fromLink ? lineupFromLink(app, mod, params) : lineupFromIds(app, mod, params); }
    catch (e) { return showBroken(root, String(e.message || e)); }
    const { entries } = lineup;
    let seed = lineup.seed;
    const favIndex = (() => { const i = entries.findIndex((e) => e.kind === "yours" && e.id === lib.selected); return i >= 0 ? i : entries.findIndex((e) => e.kind === "yours"); })();
    const fav = favIndex >= 0 ? entries[favIndex] : null;
    const saved = new Map(); // entry index -> saved Nyborg id

    let session = null, player = null, finished = false;
    const title = el("h1", { tabindex: "-1" });
    const stage = el("div", { class: "match-stage" });
    const result = el("div", { "aria-live": "polite" });
    const saveBox = el("div");
    const side = el("aside", { class: "match-side" }, result, saveBox);
    const tag = (e) => el("span", { class: `tag ${e.kind}`, text: TAGS[e.kind] });
    const shareUrl = () => new URL(`#/watch?${mod.query(engine, lineup.match(seed))}`, location.href).href;

    function run() {
      player?.stop();
      session?.free();
      finished = false;
      history.replaceState(null, "", lineup.hash(seed));
      title.textContent = `${mod.title} · seed ${seed}`;
      try {
        session = mod.start(engine, { ...lineup.match(seed), you: entries.map((e, i) => (e.kind === "yours" ? i : -1)).filter((i) => i >= 0) });
      } catch (e) {
        session = null;
        fill(stage, el("p", { class: "status bad", text: `This match can't start: ${e.message || e}` }));
        return;
      }
      player = mountPlayer(stage, session, {
        label: `${mod.title}: ${entries.map((e) => e.n.name).join(", ")}`,
        speed: player?.speed || 1, onTick: renderBoard, onFinish: renderFinish, onRestart: run,
      });
      renderBoard(session);
      window.__watch = { get session() { return session; }, get finished() { return finished; }, entries, get seed() { return seed; }, shareUrl, fromLink };
    }

    function renderBoard(s) {
      if (finished) return;
      const rows = s.board();
      fill(result,
        el("h2", { text: rows.some((r) => r.place) ? "Standings" : "Health" }),
        el("ol", { class: "board" }, rows.map((r) => el("li", { class: entries[r.slot]?.kind === "yours" ? "mine" : "" },
          el("span", { class: "swatch", style: `background:${r.hair}` }),
          r.place ? el("strong", { class: "place", text: `${r.place}.` }) : null,
          el("span", { class: "nm", text: r.name }),
          el("span", { class: "muted small note", text: r.note }),
          r.hp != null ? el("span", { class: "hp" }, el("span", { style: `width:${Math.round(r.hp * 100)}%` })) : null))),
        el("p", { class: "muted small", text: mod.blurb }));
    }

    function renderFinish(res) {
      finished = true;
      const name = (slot) => entries[slot].n.name;
      const favPlace = fav ? res.place(favIndex) : null;
      const won = res.winner != null && entries[res.winner].kind === "yours";
      const headline = res.winner == null ? res.headline : won || !fav ? `🏆 ${res.headline}` : `${name(res.winner)} wins this one.`;
      const lede = !fav ? (fromLink ? "Like one of these Nyborgs? Save it to yours and race it yourself." : "The house Nyborgs had the arena to themselves.")
        : res.winner == null ? `${fav.n.name} held on for a draw.`
        : favPlace === 1 ? `${fav.n.name} took it in ${(res.time(favIndex) ?? res.seconds).toFixed(1)}s. Nicely built.`
        : `${fav.n.name} finished ${ordinal(favPlace)} of ${entries.length}. Tweak the build and go again?`;
      // Share pack: invite copy is exactly "Race my Nyborg". The URL is the viewer query
      // (#/watch?game=…&cars|blue=…&looks=…&nameN=…), so a guest with an empty library still
      // gets the sharer's lineup — not the in-app ?slots=<id> hash (unknown ids drop to house).
      const url = shareUrl();
      const shareInput = el("input", { class: "share-url", id: "share-url", value: url, readonly: true,
        "aria-label": "Race my Nyborg link", onfocus: (e) => e.target.select() });
      const copyBtn = el("button", { type: "button", class: "btn primary", id: "share", text: "Race my Nyborg" });
      copyBtn.addEventListener("click", async () => {
        shareInput.select();
        try { await navigator.clipboard.writeText(url); copyBtn.textContent = "Copied!"; }
        catch { /* input stays selectable for long-press copy */ }
      });
      fill(result, el("section", { class: "finish-card", "aria-labelledby": "finish-h" },
        el("p", { class: "eyebrow", text: "Finish" }),
        el("h2", { id: "finish-h", tabindex: "-1", text: headline }),
        el("p", { class: "lede", text: lede }),
        el("ol", { class: "podium" }, res.order.map((slot) => el("li", { class: slot === res.winner ? "winner" : "" },
          face(entries[slot].n, { scale: 0.7, y: 0.7 }, { class: "podium-face" }),
          el("span", { class: "place", text: res.place(slot) ? ordinal(res.place(slot)) : "–" }),
          el("span", { class: "nm", text: name(slot) }), tag(entries[slot]),
          el("span", { class: "muted small", text: res.detail(slot) })))),
        el("div", { class: "row gap finish-acts" },
          el("button", { type: "button", class: "btn primary", id: "again", text: "Again ▶", title: "Same seed, same match", onclick: run }),
          el("button", { type: "button", class: "btn", id: "new-seed", text: "🎲 New seed", onclick: () => { seed = freshSeed(); run(); } }),
          fav ? el("a", { class: "btn", id: "tweak", href: app.tweakHref(fav.id), text: `Tweak ${fav.n.name}`, onclick: () => app.safe(() => lib.select(fav.id)) }) : null,
          lineup.lineupHref ? el("a", { class: "btn", href: lineup.lineupHref(seed), text: "Change lineup" }) : null),
        el("div", { class: "share-pack" },
          el("label", { class: "h3", for: "share-url", text: "Race my Nyborg" }),
          shareInput,
          copyBtn,
          el("p", { class: "muted small", text: "Anyone who opens this link watches your lineup — looks, names and builds — even with no Nyborgs of their own." }))));
      result.querySelector("h2")?.focus?.({ preventScroll: true });
      result.querySelector(".finish-card")?.scrollIntoView?.({ block: "nearest", behavior: "smooth" });
    }

    // Save from a link: each of their Nyborgs with a name or a look can join yours.
    function saveEntry(i) {
      const e = entries[i];
      const base = createNyborg(engine, e.n.name);
      const n = touch({ ...base, look: { ...e.n.look, accessories: [...e.n.look.accessories] }, builds: { ...base.builds, [mod.id]: e.slot.build } });
      if (!app.safe(() => lib.save(n))) return;
      saved.set(i, n.id);
      app.banner(e.fellBack
        ? `Saved ${n.name}. Its ${mod.title} build in the link wasn't valid, so it got the default build; tweak it any time.`
        : `Saved ${n.name} to your Nyborgs, with the link's ${mod.title} build. Other arenas start balanced.`, e.fellBack ? "" : "ok");
      renderSaves();
    }
    function renderSaves() {
      const offers = entries.map((e, i) => [e, i]).filter(([e]) => e.kind === "theirs" && (e.slot.name || e.slot.look));
      if (!offers.length) { fill(saveBox); return; }
      fill(saveBox, el("section", { class: "save-box", "aria-labelledby": "save-h" },
        el("h2", { id: "save-h", text: "From the link" }),
        el("ul", { class: "save-list" }, offers.map(([e, i]) => el("li", {},
          face(e.n, { scale: 0.6, y: 0.7 }, { class: "mini-face" }),
          el("span", { class: "nm", text: e.n.name }),
          saved.has(i)
            ? el("a", { class: "btn small", href: href(["nyborg", saved.get(i)]), text: "Saved ✓ · Edit" })
            : el("button", { type: "button", class: "btn small primary save", "data-slot": String(i), text: `Save ${e.n.name} to my Nyborgs`, onclick: () => saveEntry(i) })))),
        el("p", { class: "muted small", text: "Saving copies its build for this arena (checked by the engine), its look and its name. Nothing changes for whoever sent the link." })));
    }

    root.replaceChildren(
      el("div", { class: "intro" },
        el("p", { class: "eyebrow" },
          lineup.lineupHref ? el("a", { href: lineup.lineupHref(seed), text: "← Lineup" }) : null, lineup.lineupHref ? " · " : null,
          el("a", { href: href([]), text: "Your Nyborgs" })),
        title,
        lineup.notes.length ? el("p", { class: "link-notes muted small" }, lineup.notes.join(" ")) : null),
      el("div", { class: "watch" }, stage, side));
    renderSaves();
    run();
    return () => { player?.stop(); session?.free(); session = null; };
  },
};

function showBroken(root, why) {
  fill(root, el("div", { class: "intro" },
    el("h1", { tabindex: "-1", text: "That match link didn't work" }),
    el("p", { class: "status bad", text: why }),
    el("p", {}, el("a", { class: "btn primary", href: href([]), text: "Go to your Nyborgs" }))));
}
