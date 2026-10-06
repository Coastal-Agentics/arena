# 2026-10-06 — Why the Charger wins (and a proposed fix)

*By Blitzwing, Tank Designer-Developer.*

**What:** last night's evolution run produced a Charger that beats the starting tanks 99.35% of the time, so it's being held back as experimental. Nye asked why, and what we should change. This note is the short version. The full proposal is `docs/design/tank-balance-2026-10.md`, and nothing in the game changes until Nye says yes.
- **It isn't the build.** The champion spends its 9 points as Attack 3, Speed 4, Defense 2. Give that same build to the ordinary scripted Charger and it wins about half its fights. Give the champion's evolved settings to almost any build with Speed 3 or more and it wins 90% or more.
- **It's the charge.** Evolution pushed the champion's stop distance down to 20 units. Two tanks touch at 32, so it never stops: it keeps driving into its target, like a bulldozer. Its steering stays loose, and it dodges as often as the rules allow. That's how it closes through fire. Then Kiters and Snipers can't get away to shoot from range. Put those approach settings back to normal and it falls from 96% to 41% against the preset tanks.
- **What we tried:** 22 variants, each played 1,000 times per pairing across the four presets with every behavior, plus the new champion and two older ones, about 105,000 matches each.
  - Taking reload off Speed stops the champion, but slow Brawler Chargers take over and the Scout becomes useless.
  - A lower dodge cap for Chargers works on paper, but it breaks the Kiter > Charger > Sniper counter triangle.
  - A stat table for HP or speed barely moves the champion.
- **Proposal:** treat the Charger's approach the way we already treat its dodge. Evolution may make a Charger more careful than the scripted one, never more reckless: it stops no closer than 60 units and steers no looser than 0.2. With that, the best tank in the field wins 63.9%, every preset wins at least 52% with its best behavior, and the triangle doesn't move. Stats, presets and scripted tanks stay exactly as they are.
- **One honest catch:** evolution will always find tanks that beat the simple scripted ones. Under the change, though, no single evolved style beats the others: evolved Snipers and Kiters beat evolved Chargers. Today the bulldozer beats everything.

**Verified:** this PR changes no rules. It adds a proposal doc and an experiment program, `cargo run -p tank --release --example balance_variants`, which replays the champion's nightly score exactly (5961 of 6000) and matches the real duel code. Tests, `cargo fmt --check` and clippy pass, and every pin is unchanged (seed 42 `03722b5e86d38fac`, digest `28ae434ec1996a74`, M1 champion `d15709d4b3bd6953`).

**Why:** a tank that wins 99% isn't a strategy, it's a bug in the rules, and the arena is only fun if different builds and behaviors all have a chance.

**Next:** Nye decides. If it's a yes, the change goes in with a fresh evolution run and a re-pinned M1 champion. We did the same when we capped dodging.
