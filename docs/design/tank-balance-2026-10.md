# Tank Arena balance proposal: why the Gen 499 Charger wins, and what to change (2026-10)

**Status:** Proposal for Nye's review (2026-10-06). Nothing on `main` changes until Nye approves; this PR adds only this doc, the experiment harness and a field note.
**Author:** Blitzwing (Tank Designer-Developer). Asked for by Soundwave, approved by Nye.
**In one line:** the nightly champion wins because evolution found a Charger that never stops driving into its target, not because 3-4-2 is a broken build. The recommended fix is to give the Charger's approach the same rule #40 gave its dodge: evolution may not push it past its shipped values (stop distance at least 60 u, steering tolerance at most 0.2).

## 1. What we're looking at
- **The champion.** `web/data/evolution/champion.json`: Gen 499 of the nightly lineage (seed 1, commit `71f7c6c`), label `charger-3-4-2`. Held-out test: 5,961 of 6,000 against Gen 0 (99.35%), digest `623218a3ca642a64`, held as experimental. The harness replays it exactly (same wins, same digest), so the numbers below are about the real tank.
- **It is not a pure scripted Charger.** It carries 14 evolved params. The ones far from the scripted Charger are:

| Gene | Scripted Charger | Gen 499 | Gene range |
|---|---|---|---|
| `stop_dist` | 60 u | **20 u** (the floor) | 20–300 |
| `steer_tol` | 0.2 | **0.589** | 0.05–0.6 |
| `aim_tol` | 0.044 | 0.128 | 0.005–0.2 |
| `weave_deg` | 14° | 0° | 0–60 |
| `route_margin` | 12 u | 0.5 u | 0–40 |
| `dodge_horizon` / `dodge_margin` / `dodge_chance` | 23 / 5.3 / 0.65 | 60 / 11.7 / 0.645 | 0–60 / 0–20 / 0–0.65 |
| stall settings | 0.2 / 10 / 20 | 0.619 / 3 / 5 | |

- **The rules it plays under.** Each stat is 1–5 and the three always sum to 9. Level tables (`games/tank/src/loadout.rs`) are about ×1.2 per level: Attack sets damage (14–29), Defense sets HP (460–940), and Speed sets top speed (90–150 u/s), turn rate *and* reload (64–31 ticks). Every scripted policy has the dodge reflex; #40 capped each behavior's dodge strength at its shipped value (Charger 0.65, Kiter 0.95, Sniper 0.61). Two tanks touch at 32 u (radius 16 each).
- **The field used in every table.** The four presets (Balanced 3-3-3, Glass Cannon 5-3-1, Brawler 4-1-4, Scout 2-5-2), each with each scripted behavior (presets don't name a behavior, so all 12 play), plus three evolved champions: Gen 499 (`charger-3-4-2`), the M1 pinned champion (`charger-2-5-2`, the Scout champion) and the pre-dodge-cap nightly champion (`charger-5-3-1`, the Glass Cannon champion, archived in `web/data/evolution/archive/pre-dodge-2026-10-02/`; it predates the `dodge_chance` gene, so it plays at today's cap). 500 seeds per pairing, each played from both sides: 1,000 games per pairing, about 105,000 matches per variant. Win rate = wins ÷ games; draws count as not winning. "Against the field" = the average over the other 14 tanks.

## 2. Diagnosis
**In short:** the build is a bystander. The Gen 499 params win on almost any loadout with Speed 3 or more, and scripted tanks win normally on 3-4-2. The champion's edge is its approach: its stop distance evolved to 20 u, below the 32 u where two tanks touch, so it never stops driving into its target. Together with a loose steering tolerance and the full-strength dodge it closes through fire, and Kiters and Snipers can't open the range again. Speed matters only because the charge needs to be at least as fast as its target.

The ablations (full tables in Appendix A, 500 seeds per pairing):
1. **Behavior swap, same 3-4-2 build:** scripted Charger 52.6% against the 12 preset tanks, scripted Sniper 55.1%, scripted Kiter 18.1%. The champion: **96.4%**. Same build, ordinary results.
2. **Champion params, other builds:** 90–98% against the presets on every loadout with Speed 3, 4 or 5 (3-3-3: 92.3%), 74–75% with Speed 2, and 51–53% with Speed 1 (they can't catch Kiters or Snipers: 9–20% against them). So Speed is necessary for the strategy, but 3-4-2 isn't special.
3. **One gene put back to the scripted value:**
   - `stop_dist` 20 → 60: 96.4% → 76.4%;
   - `steer_tol` 0.589 → 0.2: → 82.5%;
   - all three approach genes (stop distance, steering, route margin) together: → **41.4%**;
   - the dodge genes, the weave genes, aim and stall: each 94–98%, little change.
4. **One champion gene added to the scripted Charger 3-4-2:** `stop_dist` 60 → 20 alone takes it from 52.6% to **90.7%**; `steer_tol` alone to 76.2%. No other single gene raises it by more than 3 points (route margin and aim alone lower it).
5. **Dodge strength:** at 0 the champion falls to 34.8% (it loses to every Kiter); at 0.4, 72.2%; at the 0.65 cap, 96.5%. The look-ahead doesn't matter (15 to 60 ticks: all about 96%). So the dodge is what lets the charge survive, but the cap #40 set is not being exceeded or gamed; the charge is new.
6. **HP and reload, one at a time (applied to every tank):** a steeper HP table (400–960) 91.8%; a flatter Speed table (105–135 u/s) 94.1%; a fixed 45-tick reload 59.1%, but that just moves the problem (section 3).

## 3. Variants
Three concrete proposals, each tested on the full field below, plus the other variants tried along the way. Every variant keeps the 9-point budget, and every one except the dodge cap leaves level 3 and the scripted params alone, so the scripted 3-3-3 counter triangle can't move.

**V1: take reload off Speed** (`reload-flat`; softer: `reload-half`). Catalog change: every build reloads in 45 ticks, so Speed buys movement only. It does break the champion (58.2%), but the slow, tanky Brawler Charger 4-1-4 takes over at 88.4%, and the Scout drops to 27.2% at its best: the stat triangle relies on Speed's reload to pay for slow tanks. Half-flattening (52/48/45/42/39) gives the same picture with smaller numbers (Brawler Charger 82.0%, Scout 33.0%). Moving reload onto Attack is worse (Scout 16.0%). **Rejected.**

**V2: lower the Charger's dodge cap** (`charger-dodge-040`). 0.65 → 0.40, for evolution and the scripted Charger, since the bound can't sit below the shipped value. On the field it nearly works (top: Glass Cannon Sniper 72.3%, champion 68.6%), but the scripted Charger collapses: Kiter > Charger goes to 90.7% and Charger > Sniper to **29.6%**, so the counter triangle breaks. At 0.50 the champion is still 79.3% and the triangle still breaks (42.8%). **Rejected.**

**V3: cap the Charger's approach at its shipped values** (`stop60+steer02`). Two gene bounds: `stop_dist` at least 60 u (today 20) and `steer_tol` at most 0.2 (today 0.6). That's the same rule #40 wrote for dodge strength ("evolution may make a tank more careful than its scripted self, never more reckless"). Scripted tanks, tables and Gen 0 don't change at all. On the field: the top build is the scripted Glass Cannon Sniper at **63.9%**, the Gen 499 champion (clamped) 35.6%, the M1 champion 48.2%, every preset 52–64% at its best behavior, the triangle unchanged at 67.1 / 68.3 / 76.2. **Recommended** (section 4).

### Headline numbers, every variant (500 seeds per pairing, both sides)
"Lowest preset" is the weakest preset at its best behavior. Kiter entries sit at or below 30% in most variants: the Kiter has been the weakest policy across loadouts since the dodge retune (BALANCE.md §2), and none of these changes target it. The triangle is the scripted 3-3-3 pairings (BALANCE.md target: each 55–80%).

| Variant | What changes | Gen 499 champion | M1 Scout champion | Top build | Lowest preset (best behavior) | Triangle K>C / C>S / S>K | Mean draws |
|---|---|---|---|---|---|---|---|
| `baseline` | Today's rules | 88.5% | 75.7% | Gen 499 champion (charger-3-4-2) 88.5% | Scout 46.2% | 67.1 / 68.3 / 76.2 | 9.3% |
| `reload-flat` | Reload fixed at 45 ticks for every build (Speed buys movement only) | 58.2% | 22.1% | Brawler Charger (4-1-4) 88.4% | Scout 27.2% | 67.1 / 68.3 / 76.2 | 9.3% |
| `reload-on-attack` | Reload table moves from Speed to Attack | 47.7% | 14.4% | Brawler Charger (4-1-4) 87.0% | Scout 16.0% | 67.1 / 68.3 / 76.2 | 8.3% |
| `reload-half` | Speed's reload table flattened by half (52/48/45/42/39) | 74.6% | 31.3% | Brawler Charger (4-1-4) 82.0% | Scout 33.0% | 67.1 / 68.3 / 76.2 | 9.4% |
| `hp-steep` | Defense table steeper (400/520/650/800/960) | 87.7% | 70.3% | Gen 499 champion (charger-3-4-2) 87.7% | Scout 46.9% | 67.1 / 68.3 / 76.2 | 8.3% |
| `speed-flat` | Speed table flattened (105–135 u/s) | 87.2% | 71.1% | Gen 499 champion (charger-3-4-2) 87.2% | Balanced 49.5% | 67.1 / 68.3 / 76.2 | 7.9% |
| `charger-dodge-040` | Charger dodge strength cap 0.65 → 0.40 | 68.6% | 50.9% | Glass Cannon Sniper (5-3-1) 72.3% | Brawler 61.5% | 90.7 / 29.6 / 76.2 ✗ | 8.5% |
| `charger-dodge-050` | Charger dodge strength cap 0.65 → 0.50 | 79.3% | 61.6% | Gen 499 champion (charger-3-4-2) 79.3% | Brawler 55.9% | 85.5 / 42.8 / 76.2 ✗ | 8.5% |
| `stop-floor-80` | Charger stop-distance gene floor 20 → 80 u | 64.0% | 69.7% | Glass Cannon Sniper (5-3-1) 75.6% | Scout 59.5% | 72.3 / 27.4 / 76.2 ✗ | 9.1% |
| `stop60+dodge050` | Stop-distance floor 60 u and Charger dodge cap 0.50 | 64.4% | 58.2% | Glass Cannon Sniper (5-3-1) 68.3% | Brawler 54.7% | 85.5 / 42.8 / 76.2 ✗ | 8.6% |
| `stop60+steer03` | Stop-distance floor 60 u and steer tolerance cap 0.3 | 43.7% | 55.5% | Glass Cannon Sniper (5-3-1) 62.1% | Brawler 49.9% | 67.1 / 68.3 / 76.2 | 9.2% |
| `stop60+steer02` | Stop-distance floor 60 u and steer tolerance cap 0.2 (both the shipped values) | 35.6% | 48.2% | Glass Cannon Sniper (5-3-1) 63.9% | Brawler 52.0% | 67.1 / 68.3 / 76.2 | 9.2% |
| `stop60+steer03+h30` | Stop floor 60 u, steer cap 0.3, dodge look-ahead cap 30 | 43.2% | 55.6% | Glass Cannon Sniper (5-3-1) 62.1% | Brawler 50.2% | 67.1 / 68.3 / 76.2 | 9.2% |
| `reload-top` | Top of Speed's reload table flattened (… 45/41/38) | 82.0% | 37.7% | Gen 499 champion (charger-3-4-2) 82.0% | Scout 34.3% | 67.1 / 68.3 / 76.2 | 10.2% |
| `reload-top+stop60` | Reload top flattened and stop-distance floor 60 u | 55.7% | 31.0% | Glass Cannon Sniper (5-3-1) 69.3% | Scout 36.0% | 67.1 / 68.3 / 76.2 | 10.0% |
| `min-range-40` | No firing with an enemy closer than 40 u (tanks touch at 32 u) | 86.3% | 62.3% | Gen 499 champion (charger-3-4-2) 86.3% | Scout 43.3% | 68.0 / 56.0 / 76.2 | 19.3% |
| `min-range-50` | No firing with an enemy closer than 50 u | 52.7% | 24.4% | Glass Cannon Sniper (5-3-1) 58.9% | Scout 43.1% | 63.5 / 57.4 / 76.2 | 34.9% |
| `min-range-50+stop60` | Minimum firing range 50 u and stop-distance floor 60 u | 43.0% | 32.7% | Glass Cannon Sniper (5-3-1) 59.8% | Scout 45.7% | 63.5 / 57.4 / 76.2 | 32.5% |
| `min-range-40+stop60` | Minimum firing range 40 u and stop-distance floor 60 u | 58.0% | 50.3% | Glass Cannon Sniper (5-3-1) 59.6% | Scout 46.5% | 68.0 / 56.0 / 76.2 | 23.2% |
| `min-range-45+stop60` | Minimum firing range 45 u and stop-distance floor 60 u | 48.2% | 40.2% | Glass Cannon Sniper (5-3-1) 60.1% | Scout 46.3% | 68.1 / 55.2 / 76.2 | 29.0% |
| `stop-floor-60` | Charger stop-distance gene floor 20 → 60 u | 74.1% | 70.6% | Gen 499 champion (charger-3-4-2) 74.1% | Brawler 48.0% | 67.1 / 68.3 / 76.2 | 9.3% |
| `stop-floor-40` | Charger stop-distance gene floor 20 → 40 u | 82.4% | 71.6% | Gen 499 champion (charger-3-4-2) 82.4% | Brawler 45.5% | 67.1 / 68.3 / 76.2 | 9.6% |

Other variants, in a line each:
- **Steeper HP** and **flatter Speed table:** the champion barely moves (87–88%).
- **Top of the reload table flattened** (… 45/41/38): champion 82.0%, Scout 34.3%.
- **A stop-distance floor alone:** at 40 u, 82.4%; at 60 u, 74.1% and 70.6% for the two champions, just over the cap. At 80 u it changes the scripted Charger (it stops at 60), and Charger > Sniper falls to 27.4%.
- **A minimum firing range** (no shots with an enemy within 40–50 u): simulated by a wrapper around every policy, which is exact because the rule only reads the tick's observation. At 50 u the top is 58.9%, but draws climb from 9% to 35% on average and Charger matchups become stalemates, which breaks the BALANCE.md draw target. At 40 u the champion is still 86.3%.

### Full round robins: baseline and the three proposals
Each block opens with the ranking and the triangle; the full matrix (row's win % against column) is folded. Short names: C/K/S = scripted Charger/Kiter/Sniper on that loadout; ★ = an evolved champion. The other variants' matrices come from `round-robin 500 <variant>`.

#### `baseline`: Today's rules

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Gen 499 champion (charger-3-4-2) | 88.5% | 0.2% |
| 2 | M1 Scout champion (charger-2-5-2) | 75.7% | 2.7% |
| 3 | Glass Cannon Sniper (5-3-1) | 58.6% | 2.2% |
| 4 | Glass Cannon Charger (5-3-1) | 52.4% | 1.7% |
| 5 | Balanced Sniper (3-3-3) | 49.1% | 9.3% |
| 6 | Brawler Sniper (4-1-4) | 47.5% | 11.9% |
| 7 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 46.8% | 1.9% |
| 8 | Scout Sniper (2-5-2) | 46.2% | 12.3% |
| 9 | Brawler Charger (4-1-4) | 42.4% | 4.7% |
| 10 | Balanced Charger (3-3-3) | 40.9% | 1.9% |
| 11 | Scout Charger (2-5-2) | 38.6% | 0.1% |
| 12 | Glass Cannon Kiter (5-3-1) | 27.9% | 12.3% |
| 13 | Balanced Kiter (3-3-3) | 25.4% | 22.0% |
| 14 | Brawler Kiter (4-1-4) | 24.8% | 35.4% |
| 15 | Scout Kiter (2-5-2) | 15.5% | 21.0% |

Top: Gen 499 champion (charger-3-4-2) at 88.5%. Best behavior per preset: Balanced 49.1%, Glass Cannon 58.6%, Brawler 47.5%, Scout 46.2%. Lowest preset (best behavior): Scout 46.2%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 24 | 28 | 59 | 19 | 90 | 64 | 0 | 3 | 36 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 44 | 0 | 0 | 83 | 2 | 0 | 1 | 9 | 58 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 69 | 36 | 43 | 14 | 79 | 57 | 1 | 18 | 88 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 57 | 39 | 65 | 71 | 90 | 74 | 0 | 38 | 37 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 47 | 11 | 0 | 84 | 24 | 0 | 5 | 19 | 56 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 74 | 83 | 46 | 31 | 97 | 61 | 3 | 29 | 87 |
| C4-1-4 | 73 | 56 | 26 | 43 | 53 | 26 | – | 35 | 50 | 73 | 64 | 10 | 18 | 24 | 42 |
| K4-1-4 | 72 | 0 | 0 | 61 | 8 | 0 | 65 | – | 0 | 79 | 1 | 0 | 2 | 7 | 52 |
| S4-1-4 | 40 | 65 | 54 | 34 | 84 | 54 | 50 | 23 | – | 43 | 69 | 71 | 2 | 25 | 50 |
| C2-5-2 | 81 | 17 | 86 | 29 | 16 | 69 | 26 | 21 | 57 | – | 39 | 90 | 0 | 0 | 8 |
| K2-5-2 | 10 | 7 | 0 | 10 | 45 | 0 | 33 | 0 | 0 | 61 | – | 0 | 1 | 2 | 49 |
| S2-5-2 | 35 | 85 | 43 | 26 | 97 | 39 | 38 | 43 | 28 | 9 | 91 | – | 9 | 26 | 76 |
| ★3-4-2 | 100 | 99 | 99 | 100 | 95 | 97 | 82 | 98 | 98 | 100 | 99 | 90 | – | 38 | 44 |
| ★2-5-2 | 97 | 89 | 76 | 61 | 80 | 68 | 75 | 93 | 71 | 100 | 94 | 60 | 61 | – | 35 |
| ★5-3-1 | 62 | 42 | 11 | 61 | 44 | 13 | 58 | 47 | 50 | 91 | 51 | 4 | 56 | 64 | – |

</details>

#### `reload-flat`: Reload fixed at 45 ticks for every build (Speed buys movement only)

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Brawler Charger (4-1-4) | 88.4% | 1.4% |
| 2 | Brawler Sniper (4-1-4) | 82.7% | 7.7% |
| 3 | Glass Cannon Sniper (5-3-1) | 63.6% | 2.3% |
| 4 | Gen 499 champion (charger-3-4-2) | 58.2% | 0.3% |
| 5 | Glass Cannon Charger (5-3-1) | 55.9% | 1.6% |
| 6 | Balanced Sniper (3-3-3) | 55.0% | 9.3% |
| 7 | Balanced Charger (3-3-3) | 54.4% | 1.7% |
| 8 | Brawler Kiter (4-1-4) | 49.1% | 33.5% |
| 9 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 46.5% | 2.9% |
| 10 | Glass Cannon Kiter (5-3-1) | 31.7% | 12.1% |
| 11 | Balanced Kiter (3-3-3) | 28.6% | 22.4% |
| 12 | Scout Sniper (2-5-2) | 27.2% | 16.4% |
| 13 | M1 Scout champion (charger-2-5-2) | 22.1% | 5.0% |
| 14 | Scout Charger (2-5-2) | 10.5% | 0.1% |
| 15 | Scout Kiter (2-5-2) | 6.5% | 22.5% |

Top: Brawler Charger (4-1-4) at 88.4%. Best behavior per preset: Balanced 55.0%, Glass Cannon 63.6%, Brawler 88.4%, Scout 27.2%. Lowest preset (best behavior): Scout 27.2%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 0 | 0 | 1 | 100 | 99 | 98 | 77 | 100 | 36 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 7 | 0 | 0 | 100 | 2 | 0 | 11 | 63 | 58 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 4 | 35 | 19 | 90 | 79 | 84 | 11 | 75 | 88 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 0 | 0 | 4 | 100 | 99 | 98 | 82 | 100 | 37 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 13 | 5 | 0 | 100 | 38 | 0 | 17 | 70 | 56 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 10 | 82 | 25 | 92 | 97 | 81 | 23 | 82 | 87 |
| C4-1-4 | 100 | 93 | 95 | 100 | 87 | 90 | – | 35 | 58 | 100 | 100 | 80 | 100 | 100 | 100 |
| K4-1-4 | 100 | 3 | 0 | 100 | 31 | 2 | 65 | – | 0 | 100 | 8 | 5 | 80 | 99 | 94 |
| S4-1-4 | 99 | 75 | 80 | 96 | 93 | 75 | 42 | 47 | – | 100 | 83 | 98 | 81 | 96 | 94 |
| C2-5-2 | 0 | 0 | 9 | 0 | 0 | 7 | 0 | 0 | 0 | – | 34 | 88 | 0 | 7 | 0 |
| K2-5-2 | 1 | 0 | 0 | 1 | 13 | 0 | 0 | 0 | 0 | 66 | – | 0 | 0 | 3 | 8 |
| S2-5-2 | 1 | 72 | 16 | 1 | 92 | 19 | 1 | 14 | 2 | 11 | 79 | – | 3 | 26 | 45 |
| ★3-4-2 | 23 | 89 | 88 | 18 | 83 | 77 | 0 | 20 | 19 | 100 | 100 | 94 | – | 100 | 6 |
| ★2-5-2 | 0 | 34 | 13 | 0 | 29 | 12 | 0 | 0 | 0 | 93 | 86 | 41 | 0 | – | 0 |
| ★5-3-1 | 62 | 42 | 11 | 61 | 44 | 13 | 0 | 6 | 6 | 100 | 90 | 21 | 94 | 100 | – |

</details>

#### `charger-dodge-040`: Charger dodge strength cap 0.65 → 0.40

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Glass Cannon Sniper (5-3-1) | 72.3% | 2.2% |
| 2 | Gen 499 champion (charger-3-4-2) | 68.6% | 0.2% |
| 3 | Scout Sniper (2-5-2) | 68.4% | 6.6% |
| 4 | Balanced Sniper (3-3-3) | 63.9% | 8.8% |
| 5 | Brawler Sniper (4-1-4) | 61.5% | 12.0% |
| 6 | M1 Scout champion (charger-2-5-2) | 50.9% | 1.8% |
| 7 | Glass Cannon Kiter (5-3-1) | 44.0% | 12.2% |
| 8 | Balanced Kiter (3-3-3) | 42.0% | 21.8% |
| 9 | Brawler Kiter (4-1-4) | 36.5% | 35.4% |
| 10 | Scout Kiter (2-5-2) | 36.2% | 20.5% |
| 11 | Glass Cannon Charger (5-3-1) | 34.9% | 2.2% |
| 12 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 29.4% | 0.5% |
| 13 | Brawler Charger (4-1-4) | 29.3% | 0.4% |
| 14 | Scout Charger (2-5-2) | 26.7% | 0.2% |
| 15 | Balanced Charger (3-3-3) | 21.9% | 2.1% |

Top: Glass Cannon Sniper (5-3-1) at 72.3%. Best behavior per preset: Balanced 63.9%, Glass Cannon 72.3%, Brawler 61.5%, Scout 68.4%. Lowest preset (best behavior): Brawler 61.5%.

Triangle at 3-3-3: Kiter > Charger 90.7%, Charger > Sniper 29.6%, Sniper > Kiter 76.2% (**broken**).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 9 | 30 | 38 | 16 | 30 | 17 | 13 | 34 | 18 | 50 | 14 | 0 | 1 | 36 |
| K3-3-3 | 91 | – | 0 | 84 | 32 | 0 | 78 | 0 | 0 | 92 | 2 | 0 | 41 | 71 | 98 |
| S3-3-3 | 70 | 76 | – | 69 | 96 | 50 | 94 | 36 | 43 | 37 | 79 | 57 | 26 | 63 | 98 |
| C5-3-1 | 38 | 16 | 30 | – | 24 | 32 | 53 | 21 | 41 | 84 | 62 | 25 | 0 | 27 | 35 |
| K5-3-1 | 84 | 32 | 0 | 76 | – | 0 | 78 | 11 | 0 | 93 | 24 | 0 | 48 | 75 | 96 |
| S5-3-1 | 69 | 96 | 50 | 67 | 100 | – | 94 | 83 | 46 | 60 | 97 | 61 | 27 | 67 | 97 |
| C4-1-4 | 81 | 22 | 6 | 47 | 22 | 6 | – | 18 | 17 | 72 | 13 | 0 | 33 | 22 | 50 |
| K4-1-4 | 87 | 0 | 0 | 78 | 8 | 0 | 82 | – | 0 | 88 | 1 | 0 | 26 | 41 | 98 |
| S4-1-4 | 66 | 65 | 54 | 58 | 84 | 54 | 82 | 23 | – | 63 | 69 | 71 | 19 | 61 | 89 |
| C2-5-2 | 81 | 8 | 62 | 16 | 7 | 40 | 28 | 12 | 37 | – | 15 | 59 | 0 | 0 | 10 |
| K2-5-2 | 49 | 7 | 0 | 38 | 45 | 0 | 86 | 0 | 0 | 85 | – | 0 | 50 | 50 | 96 |
| S2-5-2 | 86 | 85 | 43 | 74 | 97 | 39 | 99 | 43 | 28 | 41 | 91 | – | 58 | 72 | 99 |
| ★3-4-2 | 100 | 59 | 74 | 100 | 52 | 72 | 66 | 74 | 80 | 100 | 50 | 41 | – | 46 | 45 |
| ★2-5-2 | 99 | 29 | 32 | 72 | 25 | 30 | 78 | 59 | 34 | 100 | 49 | 20 | 53 | – | 34 |
| ★5-3-1 | 62 | 2 | 2 | 63 | 4 | 3 | 50 | 2 | 11 | 90 | 4 | 1 | 55 | 64 | – |

</details>

#### `stop60+steer02`: Stop-distance floor 60 u and steer tolerance cap 0.2 (both the shipped values)

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Glass Cannon Sniper (5-3-1) | 63.9% | 2.0% |
| 2 | Glass Cannon Charger (5-3-1) | 59.4% | 2.1% |
| 3 | Scout Charger (2-5-2) | 53.4% | 0.3% |
| 4 | Scout Sniper (2-5-2) | 53.1% | 11.3% |
| 5 | Balanced Sniper (3-3-3) | 52.9% | 8.9% |
| 6 | Brawler Sniper (4-1-4) | 52.0% | 11.9% |
| 7 | Brawler Charger (4-1-4) | 48.6% | 4.7% |
| 8 | M1 Scout champion (charger-2-5-2) | 48.2% | 0.9% |
| 9 | Balanced Charger (3-3-3) | 45.8% | 2.3% |
| 10 | Glass Cannon Kiter (5-3-1) | 36.9% | 12.2% |
| 11 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 36.6% | 2.4% |
| 12 | Gen 499 champion (charger-3-4-2) | 35.6% | 0.5% |
| 13 | Brawler Kiter (4-1-4) | 34.2% | 35.4% |
| 14 | Balanced Kiter (3-3-3) | 33.7% | 21.9% |
| 15 | Scout Kiter (2-5-2) | 26.9% | 20.7% |

Top: Glass Cannon Sniper (5-3-1) at 63.9%. Best behavior per preset: Balanced 52.9%, Glass Cannon 63.9%, Brawler 52.0%, Scout 53.4%. Lowest preset (best behavior): Brawler 52.0%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 24 | 28 | 59 | 19 | 90 | 64 | 23 | 18 | 67 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 44 | 0 | 0 | 83 | 2 | 0 | 73 | 48 | 62 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 69 | 36 | 43 | 14 | 79 | 57 | 36 | 33 | 91 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 57 | 39 | 65 | 71 | 90 | 74 | 47 | 64 | 62 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 47 | 11 | 0 | 84 | 24 | 0 | 79 | 66 | 62 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 74 | 83 | 46 | 31 | 97 | 61 | 49 | 54 | 90 |
| C4-1-4 | 73 | 56 | 26 | 43 | 53 | 26 | – | 35 | 50 | 73 | 64 | 10 | 65 | 65 | 41 |
| K4-1-4 | 72 | 0 | 0 | 61 | 8 | 0 | 65 | – | 0 | 79 | 1 | 0 | 80 | 48 | 63 |
| S4-1-4 | 40 | 65 | 54 | 34 | 84 | 54 | 50 | 23 | – | 43 | 69 | 71 | 51 | 32 | 58 |
| C2-5-2 | 81 | 17 | 86 | 29 | 16 | 69 | 26 | 21 | 57 | – | 39 | 90 | 76 | 73 | 68 |
| K2-5-2 | 10 | 7 | 0 | 10 | 45 | 0 | 33 | 0 | 0 | 61 | – | 0 | 76 | 70 | 64 |
| S2-5-2 | 35 | 85 | 43 | 26 | 97 | 39 | 38 | 43 | 28 | 9 | 91 | – | 71 | 58 | 78 |
| ★3-4-2 | 76 | 26 | 64 | 52 | 21 | 50 | 34 | 19 | 49 | 24 | 24 | 28 | – | 14 | 15 |
| ★2-5-2 | 81 | 51 | 66 | 36 | 34 | 46 | 35 | 51 | 64 | 26 | 30 | 38 | 86 | – | 32 |
| ★5-3-1 | 27 | 38 | 8 | 31 | 38 | 10 | 58 | 36 | 42 | 31 | 36 | 4 | 85 | 67 | – |

</details>

## 4. Recommendation: V3, the Charger approach cap
**Change:** in `games/tank/src/evolve/genome.rs`, the Charger's `stop_dist` gene floor goes from 20 to 60 u and its `steer_tol` gene ceiling from 0.6 to 0.2: both the shipped scripted values.

**Why this one:**
- **It hits the cause.** The diagnosis puts the champion's edge in its approach genes (put back, 96% → 41%). Stats, HP, reload and dodge are each either not the cause or too blunt.
- **It meets the target with room to spare.** No tank on the field above 63.9% (target: roughly 65–70%), no preset below 52.0% at its best behavior (target: none below 30%), and the champion drops to 35.6%.
- **It keeps what works.** The counter triangle, the level tables, the 9-point budget, BALANCE.md, scripted behavior, Gen 0 and every replay are untouched, because it changes only what evolution may produce.
- **It's legible and has a precedent.** It's one sentence: evolution may make a Charger more careful than its scripted self, never more reckless. That's the rule we already use for dodge strength (#40, `DODGE_CHANCE_MAX`).
- **Why 0.2 and not 0.3:** a 0.3 steering cap (`stop60+steer03`) also passes (top 62.1%, champion 43.7%, lowest preset 49.9%), but 0.2 is the shipped value, so the rule stays one sentence, and its re-tuned Charger is weaker (next section).

**Will evolution just find the next hole? (re-tuning check).** The champion was evolved under today's bounds, so a clamped champion flatters any fix. To see what evolution would do next, the harness re-tunes four starting tanks under each variant: the Gen 499 champion, the M1 champion, the scripted Glass Cannon Sniper and the scripted Balanced Kiter. Each gets 300 hill-climb steps against Gen 0, using evolution's fitness and mutations: a quarter of the genes by 10% of their range, plus a neighbouring loadout 30% of the time. The re-tuned tanks then join the round robin (☆).
- **Today's rules:** re-tuned Chargers reach fitness 1.000 and stay on top. Among evolved tanks, the Gen 499 champion beats the re-tuned Sniper 70% and the re-tuned Kiter 86%; the best re-tuned Charger (1-4-4) beats them 77% and 99%. This is the real imbalance: the best evolved tank is always a bulldozing Charger.
- **With V3:** the best re-tuned Charger tops out at 68.2% against the field. The evolved tanks form a loop instead of a ladder: the re-tuned Sniper beats the re-tuned Chargers 89–95%, and the re-tuned Kiter beats them 100% and wins or draws against the Sniper.
- **What doesn't go away:** any behavior re-tuned against Gen 0 beats the scripted tanks, under every variant including today's. Re-tuned Snipers reach 0.92–0.96 fitness: 68% against the field today, about 81% under V3 once the Chargers stop beating them. So the nightly's 70% hold line (win rate against Gen 0) will keep flagging new champions after this change too. That says Gen 0 is an easy benchmark, not that a build is broken. A suggestion for Soundwave, outside this proposal: judge promotion against a field that includes past champions as well as Gen 0.

#### `baseline`: Today's rules

Re-tuned tanks (300 hill-climb steps each against Gen 0 under this variant; training fitness = evolution's points per match):

- from Gen 499 champion: **charger-2-5-2**, fitness 1.000, params `{"aim_tol":0.045,"dodge_chance":0.649,"dodge_horizon":44.576,"dodge_margin":20.0,"route_margin":0.109,"stall.min_speed":0.995,"stall.reverse_ticks":7,"stall.stuck_ticks":8,"steer_tol":0.541,"stop_dist":21.774,"weave_deg":17.842,"weave_jitter":9,"weave_period":43,"weave_until":312.137}`
- from M1 Scout champion: **charger-1-4-4**, fitness 1.000, params `{"aim_tol":0.079,"dodge_chance":0.64,"dodge_horizon":46.558,"dodge_margin":10.691,"route_margin":37.494,"stall.min_speed":0.614,"stall.reverse_ticks":9,"stall.stuck_ticks":3,"steer_tol":0.6,"stop_dist":20.0,"weave_deg":24.043,"weave_jitter":4,"weave_period":81,"weave_until":82.868}`
- from Glass Cannon Sniper: **sniper-5-2-2**, fitness 0.918, params `{"aim_tol":0.046,"arrive_radius":10.095,"blind_ticks":101,"dodge_chance":0.586,"dodge_horizon":16.332,"dodge_margin":8.411,"evade_dist":225.25,"evade_jitter":10,"evade_ticks":10,"grid_step":18.307,"obstacle_clearance":7.31,"peek_offset":24.774,"replan_every":17,"replan_gain":193.868,"route_margin":16.37,"stall.min_speed":0.05,"stall.reverse_ticks":21,"stall.stuck_ticks":15,"steer_tol":0.141,"wall_margin":64.028}`
- from Balanced Kiter: **kiter-5-2-2**, fitness 0.759, params `{"aim_tol":0.029,"bend_deg":9.082,"dodge_chance":0.95,"dodge_horizon":11.339,"dodge_margin":1.353,"flip_every":75,"flip_jitter":63,"max_dist":534.063,"min_dist":377.011,"stall.min_speed":0.166,"stall.reverse_ticks":18,"stall.stuck_ticks":3,"steer_tol":0.301,"wall_flip_cooldown":40,"wall_margin":22.534}`

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Gen 499 champion (charger-3-4-2) | 87.6% | 0.3% |
| 2 | Re-tuned from M1 Scout champion (charger-1-4-4) | 85.0% | 1.4% |
| 3 | Re-tuned from Gen 499 champion (charger-2-5-2) | 75.3% | 0.3% |
| 4 | M1 Scout champion (charger-2-5-2) | 74.4% | 2.5% |
| 5 | Re-tuned from Glass Cannon Sniper (sniper-5-2-2) | 68.0% | 3.9% |
| 6 | Re-tuned from Balanced Kiter (kiter-5-2-2) | 49.9% | 27.4% |
| 7 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 48.6% | 1.5% |
| 8 | Glass Cannon Sniper (5-3-1) | 47.2% | 4.8% |
| 9 | Glass Cannon Charger (5-3-1) | 42.2% | 1.4% |
| 10 | Brawler Sniper (4-1-4) | 41.1% | 14.7% |
| 11 | Balanced Sniper (3-3-3) | 39.5% | 12.3% |
| 12 | Scout Sniper (2-5-2) | 37.3% | 14.4% |
| 13 | Brawler Charger (4-1-4) | 33.6% | 3.6% |
| 14 | Balanced Charger (3-3-3) | 32.0% | 1.5% |
| 15 | Scout Charger (2-5-2) | 30.1% | 0.1% |
| 16 | Glass Cannon Kiter (5-3-1) | 22.1% | 9.6% |
| 17 | Brawler Kiter (4-1-4) | 20.3% | 28.5% |
| 18 | Balanced Kiter (3-3-3) | 20.0% | 18.7% |
| 19 | Scout Kiter (2-5-2) | 12.1% | 21.0% |

Top: Gen 499 champion (charger-3-4-2) at 87.6%. Best behavior per preset: Balanced 39.5%, Glass Cannon 47.2%, Brawler 41.1%, Scout 37.3%. Lowest preset (best behavior): Scout 37.3%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 | ☆C2-5-2 | ☆C1-4-4 | ☆S5-2-2 | ☆K5-2-2 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 24 | 28 | 59 | 19 | 90 | 64 | 0 | 3 | 36 | 0 | 0 | 4 | 0 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 44 | 0 | 0 | 83 | 2 | 0 | 1 | 9 | 58 | 0 | 0 | 3 | 0 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 69 | 36 | 43 | 14 | 79 | 57 | 1 | 18 | 88 | 0 | 0 | 22 | 0 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 57 | 39 | 65 | 71 | 90 | 74 | 0 | 38 | 37 | 20 | 1 | 5 | 0 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 47 | 11 | 0 | 84 | 24 | 0 | 5 | 19 | 56 | 0 | 0 | 6 | 0 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 74 | 83 | 46 | 31 | 97 | 61 | 3 | 29 | 87 | 1 | 1 | 28 | 0 |
| C4-1-4 | 73 | 56 | 26 | 43 | 53 | 26 | – | 35 | 50 | 73 | 64 | 10 | 18 | 24 | 42 | 4 | 0 | 7 | 0 |
| K4-1-4 | 72 | 0 | 0 | 61 | 8 | 0 | 65 | – | 0 | 79 | 1 | 0 | 2 | 7 | 52 | 0 | 0 | 18 | 0 |
| S4-1-4 | 40 | 65 | 54 | 34 | 84 | 54 | 50 | 23 | – | 43 | 69 | 71 | 2 | 25 | 50 | 3 | 0 | 71 | 0 |
| C2-5-2 | 81 | 17 | 86 | 29 | 16 | 69 | 26 | 21 | 57 | – | 39 | 90 | 0 | 0 | 8 | 0 | 0 | 2 | 0 |
| K2-5-2 | 10 | 7 | 0 | 10 | 45 | 0 | 33 | 0 | 0 | 61 | – | 0 | 1 | 2 | 49 | 0 | 0 | 1 | 0 |
| S2-5-2 | 35 | 85 | 43 | 26 | 97 | 39 | 38 | 43 | 28 | 9 | 91 | – | 9 | 26 | 76 | 0 | 1 | 23 | 0 |
| ★3-4-2 | 100 | 99 | 99 | 100 | 95 | 97 | 82 | 98 | 98 | 100 | 99 | 90 | – | 38 | 44 | 100 | 82 | 70 | 86 |
| ★2-5-2 | 97 | 89 | 76 | 61 | 80 | 68 | 75 | 93 | 71 | 100 | 94 | 60 | 61 | – | 35 | 84 | 88 | 33 | 76 |
| ★5-3-1 | 62 | 42 | 11 | 61 | 44 | 13 | 58 | 47 | 50 | 91 | 51 | 4 | 56 | 64 | – | 80 | 43 | 41 | 55 |
| ☆C2-5-2 | 100 | 100 | 100 | 79 | 100 | 99 | 96 | 100 | 97 | 100 | 100 | 100 | 0 | 16 | 20 | – | 4 | 51 | 94 |
| ☆C1-4-4 | 100 | 100 | 98 | 99 | 100 | 99 | 100 | 100 | 100 | 100 | 100 | 79 | 16 | 12 | 57 | 96 | – | 77 | 99 |
| ☆S5-2-2 | 96 | 95 | 78 | 95 | 94 | 72 | 93 | 82 | 28 | 98 | 98 | 77 | 29 | 61 | 59 | 46 | 22 | – | 0 |
| ☆K5-2-2 | 100 | 73 | 10 | 100 | 99 | 44 | 100 | 82 | 2 | 100 | 18 | 32 | 14 | 23 | 45 | 6 | 1 | 46 | – |

</details>

#### `stop60+steer02`: Stop-distance floor 60 u and steer tolerance cap 0.2 (both the shipped values)

Re-tuned tanks (300 hill-climb steps each against Gen 0 under this variant; training fitness = evolution's points per match):

- from Gen 499 champion: **charger-2-5-2**, fitness 0.963, params `{"aim_tol":0.089,"dodge_chance":0.638,"dodge_horizon":58.465,"dodge_margin":14.948,"route_margin":1.026,"stall.min_speed":0.926,"stall.reverse_ticks":13,"stall.stuck_ticks":15,"steer_tol":0.2,"stop_dist":60.0,"weave_deg":21.97,"weave_jitter":6,"weave_period":120,"weave_until":0.0}`
- from M1 Scout champion: **charger-2-5-2**, fitness 0.917, params `{"aim_tol":0.089,"dodge_chance":0.65,"dodge_horizon":51.364,"dodge_margin":17.139,"route_margin":38.131,"stall.min_speed":0.127,"stall.reverse_ticks":19,"stall.stuck_ticks":16,"steer_tol":0.2,"stop_dist":60.0,"weave_deg":5.336,"weave_jitter":15,"weave_period":15,"weave_until":215.465}`
- from Glass Cannon Sniper: **sniper-5-2-2**, fitness 0.956, params `{"aim_tol":0.048,"arrive_radius":10.132,"blind_ticks":101,"dodge_chance":0.606,"dodge_horizon":20.846,"dodge_margin":7.706,"evade_dist":233.165,"evade_jitter":11,"evade_ticks":18,"grid_step":18.307,"obstacle_clearance":6.645,"peek_offset":24.272,"replan_every":42,"replan_gain":179.193,"route_margin":18.314,"stall.min_speed":0.05,"stall.reverse_ticks":16,"stall.stuck_ticks":3,"steer_tol":0.2,"wall_margin":48.007}`
- from Balanced Kiter: **kiter-5-2-2**, fitness 0.778, params `{"aim_tol":0.044,"bend_deg":11.501,"dodge_chance":0.882,"dodge_horizon":19.457,"dodge_margin":1.353,"flip_every":74,"flip_jitter":49,"max_dist":494.543,"min_dist":377.011,"stall.min_speed":0.166,"stall.reverse_ticks":24,"stall.stuck_ticks":15,"steer_tol":0.2,"wall_flip_cooldown":43,"wall_margin":20.0}`

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Re-tuned from Glass Cannon Sniper (sniper-5-2-2) | 81.3% | 3.9% |
| 2 | Re-tuned from Balanced Kiter (kiter-5-2-2) | 70.8% | 25.0% |
| 3 | Re-tuned from Gen 499 champion (charger-2-5-2) | 68.2% | 0.5% |
| 4 | Re-tuned from M1 Scout champion (charger-2-5-2) | 59.9% | 0.4% |
| 5 | Glass Cannon Sniper (5-3-1) | 53.4% | 4.3% |
| 6 | Glass Cannon Charger (5-3-1) | 52.8% | 1.7% |
| 7 | Scout Charger (2-5-2) | 46.3% | 0.6% |
| 8 | Brawler Sniper (4-1-4) | 46.1% | 14.6% |
| 9 | Brawler Charger (4-1-4) | 45.0% | 3.7% |
| 10 | Scout Sniper (2-5-2) | 43.5% | 12.5% |
| 11 | Balanced Sniper (3-3-3) | 43.0% | 11.7% |
| 12 | M1 Scout champion (charger-2-5-2) | 40.7% | 0.9% |
| 13 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 39.1% | 2.0% |
| 14 | Balanced Charger (3-3-3) | 36.7% | 1.8% |
| 15 | Glass Cannon Kiter (5-3-1) | 31.7% | 9.6% |
| 16 | Brawler Kiter (4-1-4) | 30.8% | 28.2% |
| 17 | Gen 499 champion (charger-3-4-2) | 28.4% | 0.4% |
| 18 | Balanced Kiter (3-3-3) | 28.2% | 18.4% |
| 19 | Scout Kiter (2-5-2) | 24.0% | 19.7% |

Top: Re-tuned from Glass Cannon Sniper (sniper-5-2-2) at 81.3%. Best behavior per preset: Balanced 43.0%, Glass Cannon 53.4%, Brawler 46.1%, Scout 46.3%. Lowest preset (best behavior): Balanced 43.0%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 | ☆C2-5-2 | ☆C2-5-2 | ☆S5-2-2 | ☆K5-2-2 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 24 | 28 | 59 | 19 | 90 | 64 | 23 | 18 | 67 | 4 | 13 | 2 | 0 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 44 | 0 | 0 | 83 | 2 | 0 | 73 | 48 | 62 | 18 | 17 | 0 | 0 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 69 | 36 | 43 | 14 | 79 | 57 | 36 | 33 | 91 | 6 | 7 | 21 | 0 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 57 | 39 | 65 | 71 | 90 | 74 | 47 | 64 | 62 | 52 | 62 | 4 | 0 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 47 | 11 | 0 | 84 | 24 | 0 | 79 | 66 | 62 | 27 | 26 | 2 | 0 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 74 | 83 | 46 | 31 | 97 | 61 | 49 | 54 | 90 | 19 | 21 | 27 | 0 |
| C4-1-4 | 73 | 56 | 26 | 43 | 53 | 26 | – | 35 | 50 | 73 | 64 | 10 | 65 | 65 | 41 | 59 | 64 | 6 | 0 |
| K4-1-4 | 72 | 0 | 0 | 61 | 8 | 0 | 65 | – | 0 | 79 | 1 | 0 | 80 | 48 | 63 | 36 | 27 | 13 | 0 |
| S4-1-4 | 40 | 65 | 54 | 34 | 84 | 54 | 50 | 23 | – | 43 | 69 | 71 | 51 | 32 | 58 | 22 | 20 | 60 | 0 |
| C2-5-2 | 81 | 17 | 86 | 29 | 16 | 69 | 26 | 21 | 57 | – | 39 | 90 | 76 | 73 | 68 | 37 | 45 | 4 | 0 |
| K2-5-2 | 10 | 7 | 0 | 10 | 45 | 0 | 33 | 0 | 0 | 61 | – | 0 | 76 | 70 | 64 | 16 | 39 | 0 | 0 |
| S2-5-2 | 35 | 85 | 43 | 26 | 97 | 39 | 38 | 43 | 28 | 9 | 91 | – | 71 | 58 | 78 | 9 | 9 | 22 | 0 |
| ★3-4-2 | 76 | 26 | 64 | 52 | 21 | 50 | 34 | 19 | 49 | 24 | 24 | 28 | – | 14 | 15 | 2 | 10 | 2 | 0 |
| ★2-5-2 | 81 | 51 | 66 | 36 | 34 | 46 | 35 | 51 | 64 | 26 | 30 | 38 | 86 | – | 32 | 5 | 23 | 22 | 7 |
| ★5-3-1 | 27 | 38 | 8 | 31 | 38 | 10 | 58 | 36 | 42 | 31 | 36 | 4 | 85 | 67 | – | 44 | 56 | 25 | 67 |
| ☆C2-5-2 | 95 | 82 | 94 | 48 | 73 | 81 | 41 | 64 | 78 | 58 | 83 | 91 | 98 | 95 | 56 | – | 85 | 5 | 0 |
| ☆C2-5-2 | 87 | 83 | 93 | 38 | 74 | 78 | 35 | 73 | 80 | 52 | 61 | 91 | 90 | 77 | 43 | 14 | – | 11 | 0 |
| ☆S5-2-2 | 98 | 97 | 79 | 96 | 97 | 73 | 94 | 86 | 40 | 96 | 99 | 78 | 98 | 76 | 75 | 95 | 89 | – | 0 |
| ☆K5-2-2 | 100 | 78 | 16 | 100 | 99 | 50 | 100 | 89 | 5 | 100 | 37 | 35 | 99 | 92 | 33 | 100 | 100 | 40 | – |

</details>

#### `stop60+steer03`: Stop-distance floor 60 u and steer tolerance cap 0.3

Re-tuned tanks (300 hill-climb steps each against Gen 0 under this variant; training fitness = evolution's points per match):

- from Gen 499 champion: **charger-1-4-4**, fitness 0.958, params `{"aim_tol":0.086,"dodge_chance":0.65,"dodge_horizon":58.357,"dodge_margin":20.0,"route_margin":17.368,"stall.min_speed":0.714,"stall.reverse_ticks":12,"stall.stuck_ticks":15,"steer_tol":0.3,"stop_dist":60.0,"weave_deg":29.119,"weave_jitter":22,"weave_period":120,"weave_until":0.0}`
- from M1 Scout champion: **charger-2-5-2**, fitness 0.920, params `{"aim_tol":0.098,"dodge_chance":0.65,"dodge_horizon":39.336,"dodge_margin":15.763,"route_margin":36.366,"stall.min_speed":0.612,"stall.reverse_ticks":39,"stall.stuck_ticks":27,"steer_tol":0.3,"stop_dist":60.0,"weave_deg":0.164,"weave_jitter":20,"weave_period":15,"weave_until":51.811}`
- from Glass Cannon Sniper: **sniper-5-2-2**, fitness 0.918, params `{"aim_tol":0.046,"arrive_radius":10.095,"blind_ticks":101,"dodge_chance":0.586,"dodge_horizon":16.332,"dodge_margin":8.411,"evade_dist":225.25,"evade_jitter":10,"evade_ticks":10,"grid_step":18.307,"obstacle_clearance":7.31,"peek_offset":24.774,"replan_every":17,"replan_gain":193.868,"route_margin":16.37,"stall.min_speed":0.05,"stall.reverse_ticks":21,"stall.stuck_ticks":15,"steer_tol":0.141,"wall_margin":64.028}`
- from Balanced Kiter: **kiter-5-2-2**, fitness 0.769, params `{"aim_tol":0.048,"bend_deg":11.501,"dodge_chance":0.916,"dodge_horizon":15.464,"dodge_margin":0.931,"flip_every":74,"flip_jitter":60,"max_dist":494.543,"min_dist":377.011,"stall.min_speed":0.324,"stall.reverse_ticks":28,"stall.stuck_ticks":8,"steer_tol":0.3,"wall_flip_cooldown":36,"wall_margin":28.955}`

| Rank | Tank | Avg win % vs the field | Draws |
|---|---|---|---|
| 1 | Re-tuned from Glass Cannon Sniper (sniper-5-2-2) | 81.6% | 3.7% |
| 2 | Re-tuned from Gen 499 champion (charger-1-4-4) | 71.7% | 1.0% |
| 3 | Re-tuned from Balanced Kiter (kiter-5-2-2) | 66.3% | 28.1% |
| 4 | Re-tuned from M1 Scout champion (charger-2-5-2) | 62.2% | 0.6% |
| 5 | Glass Cannon Sniper (5-3-1) | 51.1% | 5.0% |
| 6 | Glass Cannon Charger (5-3-1) | 49.6% | 1.7% |
| 7 | Scout Charger (2-5-2) | 48.4% | 0.5% |
| 8 | M1 Scout champion (charger-2-5-2) | 46.9% | 1.0% |
| 9 | Brawler Sniper (4-1-4) | 44.4% | 14.7% |
| 10 | Brawler Charger (4-1-4) | 43.7% | 3.7% |
| 11 | Scout Sniper (2-5-2) | 43.2% | 13.6% |
| 12 | Balanced Sniper (3-3-3) | 41.6% | 12.1% |
| 13 | Balanced Charger (3-3-3) | 36.4% | 1.8% |
| 14 | Pre-dodge Glass Cannon champion (charger-5-3-1) | 36.1% | 2.1% |
| 15 | Gen 499 champion (charger-3-4-2) | 34.9% | 0.4% |
| 16 | Glass Cannon Kiter (5-3-1) | 29.9% | 9.6% |
| 17 | Brawler Kiter (4-1-4) | 29.1% | 28.4% |
| 18 | Balanced Kiter (3-3-3) | 26.1% | 18.3% |
| 19 | Scout Kiter (2-5-2) | 23.2% | 20.8% |

Top: Re-tuned from Glass Cannon Sniper (sniper-5-2-2) at 81.6%. Best behavior per preset: Balanced 41.6%, Glass Cannon 51.1%, Brawler 44.4%, Scout 48.4%. Lowest preset (best behavior): Balanced 41.6%.

Triangle at 3-3-3: Kiter > Charger 67.1%, Charger > Sniper 68.3%, Sniper > Kiter 76.2% (holds, 55–80%).

<details><summary>Full round robin (row's win % against column)</summary>

| | C3-3-3 | K3-3-3 | S3-3-3 | C5-3-1 | K5-3-1 | S5-3-1 | C4-1-4 | K4-1-4 | S4-1-4 | C2-5-2 | K2-5-2 | S2-5-2 | ★3-4-2 | ★2-5-2 | ★5-3-1 | ☆C1-4-4 | ☆C2-5-2 | ☆S5-2-2 | ☆K5-2-2 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C3-3-3 | – | 33 | 68 | 41 | 39 | 68 | 24 | 28 | 59 | 19 | 90 | 64 | 18 | 14 | 67 | 8 | 12 | 4 | 0 |
| K3-3-3 | 67 | – | 0 | 61 | 32 | 0 | 44 | 0 | 0 | 83 | 2 | 0 | 57 | 34 | 62 | 6 | 17 | 3 | 0 |
| S3-3-3 | 31 | 76 | – | 31 | 96 | 50 | 69 | 36 | 43 | 14 | 79 | 57 | 24 | 21 | 91 | 4 | 5 | 22 | 0 |
| C5-3-1 | 41 | 39 | 68 | – | 48 | 65 | 57 | 39 | 65 | 71 | 90 | 74 | 44 | 57 | 62 | 13 | 54 | 5 | 1 |
| K5-3-1 | 61 | 32 | 0 | 52 | – | 0 | 47 | 11 | 0 | 84 | 24 | 0 | 67 | 53 | 62 | 8 | 31 | 6 | 0 |
| S5-3-1 | 31 | 96 | 50 | 34 | 100 | – | 74 | 83 | 46 | 31 | 97 | 61 | 39 | 40 | 90 | 8 | 14 | 28 | 0 |
| C4-1-4 | 73 | 56 | 26 | 43 | 53 | 26 | – | 35 | 50 | 73 | 64 | 10 | 61 | 54 | 41 | 57 | 56 | 7 | 1 |
| K4-1-4 | 72 | 0 | 0 | 61 | 8 | 0 | 65 | – | 0 | 79 | 1 | 0 | 79 | 41 | 63 | 13 | 24 | 18 | 0 |
| S4-1-4 | 40 | 65 | 54 | 34 | 84 | 54 | 50 | 23 | – | 43 | 69 | 71 | 23 | 29 | 58 | 10 | 20 | 71 | 0 |
| C2-5-2 | 81 | 17 | 86 | 29 | 16 | 69 | 26 | 21 | 57 | – | 39 | 90 | 69 | 79 | 68 | 54 | 68 | 2 | 0 |
| K2-5-2 | 10 | 7 | 0 | 10 | 45 | 0 | 33 | 0 | 0 | 61 | – | 0 | 66 | 72 | 64 | 1 | 49 | 1 | 0 |
| S2-5-2 | 35 | 85 | 43 | 26 | 97 | 39 | 38 | 43 | 28 | 9 | 91 | – | 67 | 55 | 78 | 6 | 12 | 23 | 0 |
| ★3-4-2 | 81 | 42 | 76 | 55 | 32 | 61 | 38 | 21 | 76 | 31 | 33 | 33 | – | 3 | 28 | 8 | 1 | 4 | 4 |
| ★2-5-2 | 86 | 65 | 77 | 43 | 47 | 59 | 46 | 59 | 67 | 19 | 28 | 41 | 97 | – | 42 | 34 | 12 | 6 | 16 |
| ★5-3-1 | 27 | 38 | 8 | 31 | 38 | 10 | 58 | 36 | 42 | 31 | 36 | 4 | 72 | 57 | – | 24 | 48 | 22 | 66 |
| ☆C1-4-4 | 92 | 94 | 96 | 87 | 92 | 92 | 42 | 87 | 90 | 45 | 99 | 82 | 91 | 65 | 74 | – | 53 | 1 | 8 |
| ☆C2-5-2 | 87 | 83 | 94 | 46 | 69 | 85 | 43 | 76 | 80 | 27 | 51 | 87 | 99 | 87 | 51 | 46 | – | 2 | 4 |
| ☆S5-2-2 | 96 | 95 | 78 | 95 | 94 | 72 | 93 | 82 | 28 | 98 | 98 | 77 | 96 | 92 | 78 | 99 | 97 | – | 0 |
| ☆K5-2-2 | 100 | 79 | 8 | 99 | 99 | 39 | 99 | 85 | 3 | 100 | 17 | 26 | 96 | 84 | 33 | 91 | 96 | 39 | – |

</details>

### Exact implementation diff (after Nye approves)
```diff
diff --git a/games/tank/src/evolve/genome.rs b/games/tank/src/evolve/genome.rs
index acf9f3a..a0ed4ca 100644
--- a/games/tank/src/evolve/genome.rs
+++ b/games/tank/src/evolve/genome.rs
@@ -64,8 +64,8 @@ macro_rules! stall_genes {
 const CHARGER: [Gene<ChargerParams>; 14] = {
     let [s0, s1, s2] = stall_genes!(ChargerParams);
     [
-        fg!(ChargerParams, "steer_tol", steer_tol, 0.05, 0.6),
-        fg!(ChargerParams, "stop_dist", stop_dist, 20.0, 300.0),
+        fg!(ChargerParams, "steer_tol", steer_tol, 0.05, CHARGER_STEER_TOL_MAX),
+        fg!(ChargerParams, "stop_dist", stop_dist, CHARGER_STOP_DIST_MIN, 300.0),
         fg!(ChargerParams, "aim_tol", aim_tol, 0.005, 0.2),
         fg!(ChargerParams, "route_margin", route_margin, 0.0, 40.0),
         fg!(ChargerParams, "weave_deg", weave_deg, 0.0, 60.0),
@@ -168,6 +168,15 @@ const SNIPER: [Gene<SniperParams>; 20] = {
 /// `docs/fieldnotes/2026-10-02-dodge-cap.md`.
 pub const DODGE_CHANCE_MAX: [f32; 3] = [0.65, 0.95, 0.61];
 
+/// Lower bound of the Charger's `stop_dist` gene: its shipped 60 u. Below 32 u (two
+/// radii) an evolved Charger never stops driving into its target; see
+/// `docs/design/tank-balance-2026-10.md`. Like [`DODGE_CHANCE_MAX`], evolution may make
+/// a Charger more careful than its scripted self, never more reckless.
+pub const CHARGER_STOP_DIST_MIN: f32 = 60.0;
+
+/// Upper bound of the Charger's `steer_tol` gene: its shipped 0.2 (was 0.6).
+pub const CHARGER_STEER_TOL_MAX: f32 = 0.2;
+
 /// Kiter's range band keeps at least this width (`max_dist >= min_dist + KITER_BAND`).
 pub const KITER_BAND: f32 = 20.0;
```

**What else the implementation PR has to do (tested on a scratch copy of `main` with the diff above):**
- All 71 `tank` unit tests pass unchanged; the scripted Charger is inside the new bounds.
- **`games/tank/tests/evolve_m1.rs` fails**, as expected: the pinned M1 champion has `steer_tol` 0.6 and `stop_dist` 20, and `Genome::from_json` refuses out-of-bounds genes. The M1 pin `d15709d4b3bd6953` must be re-made from a new seed-1, 100-generation run (the ignored full test, about 15 CPU-minutes). That's a GATE-003 amendment, the same as the dodge-cap re-pin.
- **The nightly lineage restarts.** `web/data/evolution/{champion,genomes,state,history}.json` hold out-of-bounds Chargers. Archive them as `archive/pre-approach-cap-2026-10/`, as we did with `pre-dodge-2026-10-02`, and start again from Gen 0.
- **Unchanged:** the Chaser/Wanderer seed hashes (42 `03722b5e86d38fac`, 7 `51234f61b02b5784`, 101 `baf3fcb2cbb76c06`, u64::MAX `f1d983e88de5d020`), the bot smoke digest `28ae434ec1996a74`, the parity fixtures, `BALANCE.md` (scripted tanks only) and replay formats.
- Update `docs/design/tank-refit.md`'s "Evolution" row and the GATE-003 gene table to the new bounds.

## 5. Rerun everything
```text
cargo run -p tank --release --example balance_variants -- diagnose 500        # Appendix A, ~2 min on 8 cores
cargo run -p tank --release --example balance_variants -- round-robin 500     # every variant, ~6 min
cargo run -p tank --release --example balance_variants -- round-robin 500 --retune baseline stop60+steer02 stop60+steer03
cargo run -p tank --release --example balance_variants -- list                # variants and the field's genomes
```
Deterministic: same code, same numbers on any thread count. Seeds start at 20,000,000 for the report and 30,000,000 for re-tuning, away from training (1e6) and held-out (9e9) seeds. At start-up the harness checks that its `baseline` variant gives exactly `tank::evolve::duel`'s results, so "today's rules" really is today's rules.

## Appendix A: diagnosis tables
<details><summary>All ablations (500 seeds per pairing)</summary>

"vs Gen 0" is the nightly's held-out test (scripted Charger, Kiter and Sniper at 3-3-3) on this harness's seeds; "vs 12 preset tanks" is every preset with every scripted behavior.

The Gen 499 champion (copied from `web/data/evolution/champion.json` at `c538b47`) replays its nightly held-out test exactly: 5961 of 6000 (99.35%), digest `623218a3ca642a64` (file: 5961 wins, digest `623218a3ca642a64`).

#### 1. The real champion, and its build with other behaviors

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| Gen 499 champion | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| Scripted Charger at 3-4-2 | charger-3-4-2 | 61.6% | 80.3 / 24.6 / 79.8 | 52.6% |
| Scripted Kiter at 3-4-2 | kiter-3-4-2 | 13.7% | 34.1 / 7.0 / 0.0 | 18.1% |
| Scripted Sniper at 3-4-2 | sniper-3-4-2 | 58.8% | 38.9 / 87.3 / 50.1 | 55.1% |

#### 2. The champion's params on every loadout

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| Champion params at 1-3-5 | charger-1-3-5 | 97.9% | 100.0 / 99.4 / 94.4 | 90.2% |
| Champion params at 1-4-4 | charger-1-4-4 | 99.8% | 100.0 / 99.7 / 99.8 | 97.5% |
| Champion params at 1-5-3 | charger-1-5-3 | 99.1% | 99.9 / 99.7 / 97.8 | 97.9% |
| Champion params at 2-2-5 | charger-2-2-5 | 83.4% | 100.0 / 88.3 / 61.9 | 74.7% |
| Champion params at 2-3-4 | charger-2-3-4 | 98.6% | 100.0 / 99.0 / 96.7 | 92.7% |
| Champion params at 2-4-3 | charger-2-4-3 | 99.6% | 100.0 / 99.3 / 99.6 | 98.4% |
| Champion params at 2-5-2 | charger-2-5-2 | 99.0% | 99.9 / 99.6 / 97.6 | 96.9% |
| Champion params at 3-1-5 | charger-3-1-5 | 40.6% | 99.5 / 13.6 / 8.6 | 51.5% |
| Champion params at 3-2-4 | charger-3-2-4 | 86.4% | 100.0 / 87.6 / 71.6 | 74.2% |
| Champion params at 3-3-3 | charger-3-3-3 | 97.8% | 100.0 / 98.0 / 95.4 | 92.3% |
| Champion params at 3-4-2 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| Champion params at 3-5-1 | charger-3-5-1 | 98.3% | 99.8 / 98.9 / 96.3 | 97.3% |
| Champion params at 4-1-4 | charger-4-1-4 | 43.2% | 99.4 / 17.1 / 13.1 | 51.6% |
| Champion params at 4-2-3 | charger-4-2-3 | 82.7% | 100.0 / 80.0 / 68.0 | 74.0% |
| Champion params at 4-3-2 | charger-4-3-2 | 97.3% | 100.0 / 97.3 / 94.6 | 90.9% |
| Champion params at 4-4-1 | charger-4-4-1 | 98.1% | 99.9 / 96.7 / 97.6 | 97.2% |
| Champion params at 5-1-3 | charger-5-1-3 | 45.0% | 99.1 / 20.2 / 15.8 | 52.9% |
| Champion params at 5-2-2 | charger-5-2-2 | 84.4% | 100.0 / 80.4 / 72.9 | 73.8% |
| Champion params at 5-3-1 | charger-5-3-1 | 96.5% | 99.8 / 95.3 / 94.3 | 93.2% |

#### 3. Champion with one gene put back to the scripted default

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| steer_tol 0.589 → 0.2 | charger-3-4-2 | 95.9% | 99.8 / 96.9 / 91.0 | 82.5% |
| stop_dist 20 → 60 | charger-3-4-2 | 85.8% | 93.9 / 74.2 / 89.2 | 76.4% |
| aim_tol 0.128 → 0.044 | charger-3-4-2 | 100.0% | 100.0 / 100.0 / 99.9 | 98.3% |
| route_margin 0.522 → 12 | charger-3-4-2 | 94.8% | 99.8 / 91.4 / 93.2 | 92.2% |
| weave_deg 0 → 14 | charger-3-4-2 | 97.8% | 99.7 / 96.3 / 97.3 | 94.3% |
| weave_period 120 → 30 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| weave_jitter 12 → 10 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| weave_until 12.916 → 150 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| dodge_horizon 60 → 23 | charger-3-4-2 | 99.2% | 100.0 / 98.4 / 99.3 | 96.3% |
| dodge_margin 11.738 → 5.3 | charger-3-4-2 | 98.6% | 100.0 / 97.7 / 98.2 | 96.4% |
| dodge_chance 0.645 → 0.65 | charger-3-4-2 | 99.4% | 100.0 / 98.7 / 99.5 | 96.5% |
| stall.min_speed 0.619 → 0.2 | charger-3-4-2 | 98.8% | 100.0 / 98.2 / 98.2 | 95.7% |
| stall.stuck_ticks 3 → 10 | charger-3-4-2 | 99.1% | 99.5 / 98.7 / 99.1 | 94.5% |
| stall.reverse_ticks 5 → 20 | charger-3-4-2 | 98.9% | 98.9 / 99.1 / 98.7 | 97.8% |

#### 4. Scripted Charger 3-4-2 with one champion gene added

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| steer_tol 0.2 → 0.589 | charger-3-4-2 | 88.2% | 85.4 / 90.9 / 88.3 | 76.2% |
| stop_dist 60 → 20 | charger-3-4-2 | 98.0% | 97.6 / 100.0 / 96.3 | 90.7% |
| aim_tol 0.044 → 0.128 | charger-3-4-2 | 63.7% | 81.0 / 41.4 / 68.6 | 47.1% |
| route_margin 12 → 0.522 | charger-3-4-2 | 51.8% | 76.1 / 8.3 / 71.0 | 45.1% |
| weave_deg 14 → 0 | charger-3-4-2 | 57.0% | 70.5 / 26.3 / 74.3 | 50.3% |
| weave_period 30 → 120 | charger-3-4-2 | 63.6% | 81.6 / 27.6 / 81.7 | 55.2% |
| weave_jitter 10 → 12 | charger-3-4-2 | 60.8% | 80.4 / 21.3 / 80.7 | 53.0% |
| weave_until 150 → 12.916 | charger-3-4-2 | 64.2% | 75.7 / 34.5 / 82.4 | 54.6% |
| dodge_horizon 23 → 60 | charger-3-4-2 | 63.1% | 81.8 / 26.0 / 81.4 | 54.0% |
| dodge_margin 5.3 → 11.738 | charger-3-4-2 | 62.5% | 79.4 / 27.8 / 80.2 | 54.0% |
| dodge_chance 0.65 → 0.645 | charger-3-4-2 | 61.1% | 80.1 / 23.9 / 79.3 | 52.1% |
| stall.min_speed 0.2 → 0.619 | charger-3-4-2 | 62.2% | 80.3 / 25.8 / 80.5 | 53.2% |
| stall.stuck_ticks 10 → 3 | charger-3-4-2 | 63.0% | 80.3 / 27.7 / 81.1 | 53.9% |
| stall.reverse_ticks 20 → 5 | charger-3-4-2 | 60.3% | 80.3 / 22.1 / 78.5 | 51.2% |

#### 5. Champion with one gene group put back

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| dodge (horizon, margin, chance) | charger-3-4-2 | 98.7% | 99.9 / 98.2 / 98.1 | 96.4% |
| approach (stop_dist, steer_tol, route_margin) | charger-3-4-2 | 56.0% | 72.8 / 37.8 / 57.5 | 41.4% |
| weave (deg, period, jitter, until) | charger-3-4-2 | 98.7% | 100.0 / 98.1 / 98.1 | 94.1% |
| aim_tol | charger-3-4-2 | 100.0% | 100.0 / 100.0 / 99.9 | 98.3% |
| stall (3) | charger-3-4-2 | 98.3% | 99.1 / 98.7 / 97.1 | 96.3% |

#### 6. Champion's dodge strength and look-ahead

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| dodge_chance 0 | charger-3-4-2 | 40.7% | 99.9 / 0.1 / 22.0 | 34.8% |
| dodge_chance 0.25 | charger-3-4-2 | 54.9% | 100.0 / 18.8 / 46.0 | 51.4% |
| dodge_chance 0.4 | charger-3-4-2 | 77.6% | 100.0 / 58.8 / 74.0 | 72.2% |
| dodge_chance 0.5 | charger-3-4-2 | 90.3% | 100.0 / 82.6 / 88.3 | 85.3% |
| dodge_chance 0.65 | charger-3-4-2 | 99.4% | 100.0 / 98.7 / 99.5 | 96.5% |
| dodge_horizon 0 | charger-3-4-2 | 40.7% | 99.9 / 0.1 / 22.0 | 34.8% |
| dodge_horizon 15 | charger-3-4-2 | 99.2% | 100.0 / 98.5 / 99.2 | 96.0% |
| dodge_horizon 23 | charger-3-4-2 | 99.2% | 100.0 / 98.4 / 99.3 | 96.3% |
| dodge_horizon 30 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.2 | 96.4% |
| dodge_horizon 45 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |
| dodge_horizon 60 | charger-3-4-2 | 99.3% | 100.0 / 98.6 / 99.4 | 96.4% |

#### 7. Rule ablations (each applied to every tank; Gen 0 is 3-3-3)

| Tank | Genome | vs Gen 0 | vs Charger / Kiter / Sniper | vs 12 preset tanks |
|---|---|---|---|---|
| Reload fixed at 45 ticks for every build (Speed buys movement only) | charger-3-4-2 | 66.7% | 22.6 / 89.0 / 88.4 | 59.1% |
| Reload table moves from Speed to Attack | charger-3-4-2 | 66.7% | 22.6 / 89.0 / 88.4 | 47.3% |
| Speed's reload table flattened by half (52/48/45/42/39) | charger-3-4-2 | 93.2% | 91.8 / 94.3 / 93.6 | 77.8% |
| Defense table steeper (400/520/650/800/960) | charger-3-4-2 | 97.0% | 98.6 / 95.4 / 97.1 | 91.8% |
| Speed table flattened (105–135 u/s) | charger-3-4-2 | 99.1% | 99.9 / 99.0 / 98.3 | 94.1% |
| Charger dodge strength cap 0.65 → 0.40 | charger-3-4-2 | 77.6% | 100.0 / 58.8 / 74.0 | 72.4% |
| Charger dodge strength cap 0.65 → 0.50 | charger-3-4-2 | 90.3% | 100.0 / 82.6 / 88.3 | 85.2% |
| Charger stop-distance gene floor 20 → 80 u | charger-3-4-2 | 66.4% | 97.4 / 74.2 / 27.6 | 66.6% |
| Stop-distance floor 60 u and Charger dodge cap 0.50 | charger-3-4-2 | 75.6% | 92.9 / 53.2 / 80.8 | 65.8% |
| Stop-distance floor 60 u and steer tolerance cap 0.3 | charger-3-4-2 | 66.5% | 81.3 / 42.2 / 76.1 | 48.4% |
| Stop-distance floor 60 u and steer tolerance cap 0.2 (both the shipped values) | charger-3-4-2 | 55.6% | 76.5 / 26.5 / 63.8 | 39.1% |
| Stop floor 60 u, steer cap 0.3, dodge look-ahead cap 30 | charger-3-4-2 | 65.2% | 81.4 / 39.2 / 75.0 | 47.8% |
| Top of Speed's reload table flattened (… 45/41/38) | charger-3-4-2 | 96.6% | 98.0 / 95.8 / 96.0 | 86.1% |
| Reload top flattened and stop-distance floor 60 u | charger-3-4-2 | 41.5% | 4.6 / 54.5 / 65.5 | 54.3% |
| No firing with an enemy closer than 40 u (tanks touch at 32 u) | charger-3-4-2 | 99.2% | 99.4 / 99.6 / 98.6 | 98.3% |
| No firing with an enemy closer than 50 u | charger-3-4-2 | 65.8% | 0.0 / 99.5 / 97.9 | 60.1% |
| Minimum firing range 50 u and stop-distance floor 60 u | charger-3-4-2 | 48.1% | 0.3 / 77.9 / 66.0 | 49.4% |
| Minimum firing range 40 u and stop-distance floor 60 u | charger-3-4-2 | 61.1% | 36.4 / 75.0 / 72.0 | 60.1% |
| Minimum firing range 45 u and stop-distance floor 60 u | charger-3-4-2 | 48.9% | 5.7 / 74.3 / 66.7 | 51.8% |
| Charger stop-distance gene floor 20 → 60 u | charger-3-4-2 | 85.8% | 93.9 / 74.2 / 89.2 | 76.4% |
| Charger stop-distance gene floor 20 → 40 u | charger-3-4-2 | 96.3% | 93.9 / 98.3 / 96.8 | 88.3% |

</details>
