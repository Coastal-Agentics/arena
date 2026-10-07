# 2026-10-07 — The Charger's approach cap is in

*By Blitzwing, Tank Designer-Developer.*

**What:** Nye approved V3 from the balance proposal (#80), so evolved Chargers can't bulldoze any more.
- **The rule:** an evolved Charger now stops at least 60 units from its target and steers no looser than 0.2. Those are the scripted Charger's own settings. The rule is the same as the dodge cap: evolution may make a tank more careful than its scripted self, never more reckless. Stats, presets and the scripted tanks don't change.
- **A new M1 champion:** the old pinned champion was a bulldozing Charger, so it no longer fits the rules. The same seed-1, 100-generation run under the new rules now produces a Sniper, `sniper-5-2-2`. It wins 95.6% against the scripted tanks (digest `0a2f3a7e2e498278`), so it's held as experimental like the others. This is the GATE-003 re-pin, and the note is in the plan's new Amendments section.
- **The nightly starts over:** the old lineage, 600 generations ending in Gen 599 `charger-3-5-1`, is archived in `web/data/evolution/archive/pre-v3-2026-10-07/`. The next nightly starts fresh at Gen 0.

**Verified:** the full 100-generation run reproduces the new pin exactly. The other pins are unchanged: seed 42 `03722b5e86d38fac`, 7 `51234f61b02b5784`, 101 `baf3fcb2cbb76c06`, u64::MAX `f1d983e88de5d020` and digest `28ae434ec1996a74`. Workspace tests, `cargo fmt --check` and clippy pass.

**Why:** a tank that wins 99% by driving into its target leaves no room for anything else. With the cap, no evolved style beats all the others.

**Next:** watch the first nights of the new lineage. Evolution still beats the simple scripted tanks easily, so judging promotion against past champions as well as Gen 0 is worth a look.
