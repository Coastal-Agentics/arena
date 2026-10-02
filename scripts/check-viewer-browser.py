#!/usr/bin/env python3
"""Headless-Chrome check that the arena viewer's Watch tab always describes the match it
is running, and that it agrees with the URL, the Customize tab and the sim.

    pip install playwright   # uses the system Chrome; set CHROME=/path/to/chrome
    python3 scripts/check-viewer-browser.py

Serves web/ under /arena/ (like GitHub Pages) on a free local port. For every scenario it
compares the URL, the page's spec, the running match's setup, the Watch cards, the
Customize readout and each tank's max_hp, and checks that no web storage is used.
Regression cases: a bad seed typed in Customize used to leave the previous match (and its
build on the Watch cards) showing; a match that ended while a Customize edit was pending
used to name the edited build in its result line. The result-line check is
winner-neutral: it reads the winner from the sim's outcome instead of assuming one.
CI runs this in the `wasm` job (.github/workflows/ci.yml) with the runner's
/usr/bin/google-chrome, after the web/pkg freshness and parity checks.
"""
import functools, http.server, os, re, sys, threading
from playwright.sync_api import sync_playwright

WEB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "web")
HP = {1: 460, 2: 550, 3: 650, 4: 790, 5: 940}  # MAX_HP by Defense level (games/tank)
TEAMS = ["Blue", "Orange"]  # TEAM_NAMES in web/arena.js, by side
fails = []


def check(ok, msg):
    print(("PASS " if ok else "FAIL ") + msg, flush=True)
    if not ok:
        fails.append(msg)


class Handler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        path = path.split("?", 1)[0]
        if path.startswith("/arena/"):
            path = path[len("/arena"):]
        return os.path.join(WEB, path.lstrip("/"))

    def log_message(self, *a):
        pass


PROBE = """() => {
  const a = window.__arena, q = new URLSearchParams(location.search);
  const text = (el) => el.innerText.replace(/\\s+/g, ' ');
  const tanks = (s) => s && s.mode === 'tank' ? s.tanks.map(t => t.behavior + '-' + t.loadout) : null;
  return {
    tab: document.getElementById('tab-watch').getAttribute('aria-selected') === 'true' ? 'watch' : 'customize',
    url: [q.get('blue'), q.get('orange')], spec: tanks(a.spec), running: tanks(a.running),
    setup: a.setup ? a.setup.tanks.map(t => t.behavior + '-' + t.loadout) : null,
    maxHp: a.state ? a.state.tanks.map(t => t.max_hp) : null,
    cards: [...document.querySelectorAll('#watch-cards .tankcard')].map(text),
    read: [0, 1].map(i => { const r = document.getElementById('cz-read' + i); return r ? text(r) : ''; }),
    result: document.getElementById('result').textContent,
    ended: !!(a.state && a.state.outcome),
    winner: a.state && a.state.outcome ? a.state.outcome.winner : null,
    storage: Object.keys(localStorage).length + Object.keys(sessionStorage).length,
  };
}"""


def build(t):
    return "Build " + t.split("-", 1)[1].replace("-", "/")


def result_names_match(s, want):
    """The result line names both running builds, with the sim's winner first (or a draw).

    Winner-neutral: who wins comes from the outcome, never from an assumed side."""
    side = [f"{TEAMS[i]} \\([^()]+ {re.escape(build(w)[len('Build '):])}\\)" for i, w in enumerate(want)]
    if not s["ended"]:
        return False
    w = s["winner"]
    if w is None:
        pat = rf"Draw \(.+\) at [0-9.]+s: {side[0]} vs {side[1]}$"
    else:
        pat = rf"{side[w]} beats {side[1 - w]} — .+, [0-9.]+s$"
    return re.match(pat, s["result"]) is not None


def agree(pg, label, want):
    """URL, spec, running match, Watch cards / Customize readout and max_hp all show `want`."""
    pg.wait_for_function("window.__arena && window.__arena.state")
    s = pg.evaluate(PROBE)
    ok = s["url"] == want and s["spec"] == want and s["storage"] == 0
    if s["tab"] == "watch":
        ok &= s["running"] == want and s["setup"] == want
        ok &= all(build(w) in c for w, c in zip(want, s["cards"]))
        ok &= s["maxHp"] == [HP[int(w[-1])] for w in want]
    else:
        ok &= all(build(w) in r for w, r in zip(want, s["read"]))
    check(ok, f"{label}: {s['tab']} url={s['url']} running={s['running']} max_hp={s['maxHp']}")
    return s


def main():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    base = f"http://127.0.0.1:{srv.server_address[1]}/arena"
    arena = base + "/arena.html"
    errors = []
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("CHROME", "/usr/bin/google-chrome"), headless=True)
        pg = b.new_page(viewport={"width": 1000, "height": 1300})
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("console", lambda m: m.type == "error" and errors.append(m.text))
        preset = lambda i, l: pg.click(f"#cz-presets{i} button[data-l='{l}']")

        pg.goto(arena)
        agree(pg, "fresh load is 3/3/3", ["kiter-3-3-3", "charger-3-3-3"])
        link = "?seed=42&blue=kiter-3-3-3&orange=charger-4-1-4"
        pg.goto(arena + link)
        agree(pg, "link", ["kiter-3-3-3", "charger-4-1-4"])
        pg.click("#tab-customize"); preset(0, "5-3-1")
        agree(pg, "Customize preset", ["kiter-5-3-1", "charger-4-1-4"])
        pg.click("#tab-watch")
        agree(pg, "preset, then the Watch tab", ["kiter-5-3-1", "charger-4-1-4"])
        pg.click("#tab-customize"); preset(0, "3-3-3"); pg.click("#watch-this")
        agree(pg, "preset, then Watch this match", ["kiter-3-3-3", "charger-4-1-4"])
        pg.click("#tab-customize"); preset(0, "2-5-2"); pg.reload()
        agree(pg, "reload with an edit pending", ["kiter-2-5-2", "charger-4-1-4"])
        pg.goto(arena + "?seed=42&blue=kiter-5-3-1&orange=charger-4-1-4")
        pg.go_back()
        agree(pg, "back", ["kiter-2-5-2", "charger-4-1-4"])
        pg.go_forward()
        agree(pg, "forward", ["kiter-5-3-1", "charger-4-1-4"])

        # Regression: a bad seed from Customize must not leave the old match (5/3/1) showing.
        pg.click("#tab-customize"); preset(0, "3-3-3"); pg.fill("#cz-seed", "42x"); pg.click("#tab-watch")
        s = pg.evaluate(PROBE)
        check(s["running"] is None and s["maxHp"] is None and "5/3/1" not in " ".join(s["cards"])
              and build("kiter-3-3-3") in s["cards"][0] and "seed" in s["result"],
              f"bad seed: no stale match or build ({s['cards'][0][:40]!r}, {s['result']!r})")
        pg.fill("#seed", "42"); pg.press("#seed", "Enter")
        agree(pg, "fixed seed recovers", ["kiter-3-3-3", "charger-4-1-4"])

        # Regression: a match that ends while a Customize edit is pending names its own builds.
        pg.goto(arena + link + "&speed=4")
        pg.wait_for_function("window.__arena && window.__arena.state")
        pg.click("#tab-customize"); preset(0, "5-3-1")
        pg.wait_for_function("window.__arena.state.outcome", timeout=180000)
        preset(0, "3-3-3"); pg.click("#tab-watch")
        s = agree(pg, "ended during an undone edit", ["kiter-3-3-3", "charger-4-1-4"])
        check("5/3/1" not in s["result"] and result_names_match(s, ["kiter-3-3-3", "charger-4-1-4"]),
              f"result line names the running builds, winner per the sim (winner={s['winner']}): {s['result']!r}")
        b.close()
    check(not errors, f"no page errors {errors}")
    print(f"check-viewer-browser: {'FAILED: ' + str(len(fails)) if fails else 'all checks passed'}")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
