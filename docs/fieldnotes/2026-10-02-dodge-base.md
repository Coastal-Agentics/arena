# 2026-10-02 — Every scripted tank dodges

*By Blitzwing, Tank Designer-Developer.*

**What:** Nye decided that dodging belongs in the base game, so the scripted Charger, Kiter and Sniper now all dodge. One shared reflex has three settings per policy: how far ahead it looks (ticks), how close a shell must pass to count as a threat, and a strength. The strength is the chance of reacting to each shell, rolled once per shell from its own seeded stream. The Kiter had no dodge before and gets one now. The evolution genome gains `dodge_chance` for all three policies and the Kiter's dodge genes. Several policy numbers were retuned with it. The level tables are unchanged.

**Result:** every balance target passes again (`games/tank/BALANCE.md` §0). Kiter beats Charger 69.2%, Charger beats Sniper 64.5% and Sniper beats Kiter 75.0%. The median match is 44.6 s and 8.3% end in draws. The strongest loadouts average 58.8% (Charger), 25.0% (Kiter) and 59.2% (Sniper). Shipped settings (look-ahead / threshold / strength): Charger 23 / 5.3 u / 0.65, Kiter 11 / 0.66 u / 0.95, Sniper 15 / 5.4 u / 0.61.

**What we learned:** a reflex that always reacts makes a tank almost untouchable at range: Kiter-vs-Sniper and both mirrors ended in 120-second draws. Hence the strengths below 1. Rolling once per threat episode instead of once per shell made the best Sniper loadout flip between all-slow and all-fast builds as the look-ahead changed. The Sniper never loses to the Kiter; its win rate is set by how many of those matches time out, and the Kiter's strafe-flip timing controls that.

**Evolution:** Gen 0 changed, so the pinned M1 champion was re-run from seed 1. Gen 99 is a Charger 4/4/1 that wins **95.5%** against the new Gen 0 (digest `92575320ebf3f47e`), so it is still held as experimental. A 10-generation run already reaches 99.9%. Evolution's first move is to raise the dodge strength to 1. The nightly's `state.json` no longer loads (the gene tables changed), and its history and champion were scored against the old Gen 0. Soundwave needs to restart the run from Gen 0 (details in the PR).

**Why:** the M1 champion won 89–99.9% only because evolution turned on a reflex that every shipped policy had off.

**Next:** Nye approves this as a GATE-002 amendment before merge. Then the nightly restarts, and Nye decides whether full-strength dodging should be capped or stay something evolution can find.
