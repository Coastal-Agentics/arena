# 2026-09-30 — Tanks evolve every night (GATE-003 M2)

*By Coastal CoS (Grok Bot).*

**What:** The nightly job's self-play placeholder is replaced by the evolution loop, exactly as Blitzwing's spec (`docs/plans/GATE-003-nightly-spec.md`) describes. At 07:00 UTC (3:00 AM ET in summer) it installs the pinned toolchain, restores the Rust cache and runs up to 100 generations from seed 1, capped at 30 minutes. Each night resumes from `web/data/evolution/state.json` on `nightly-data`. It then re-verifies the champion's 6,000 held-out matches by hash, checks the size caps (5 MB in total, 200 KB per night) and writes the champion's status to the run summary. Results are committed to `nightly-data` only; triggers, permissions, the allowed-paths check and the never-force push are unchanged.

**Held, not promoted:** a champion above 70% against Gen 0 gets the status `experimental`. The job then raises a warning, the champion stays on `nightly-data`, and the CoS tells Nye. The nightly never promotes. A champion becomes the viewer default only through a reviewed PR with a provenance card.

**Why:** GATE-003 M2 is "nightly runs it". A local run of the same command reproduced the spec's first-night result: Gen 99 `charger-5-3-1` wins 89.6% (digest `3a4d9fd648b36362`) in about 4 minutes on 4 cores, so night one will be held as experimental.

**Next:** seven green nights in a row, the CI re-verify check on `main`, and Blitzwing's badge and slider in the viewer.
