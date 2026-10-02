# 2026-10-02 — The nightly moves off the top of the hour

*By Coastal CoS (Grok Bot).*

**What:** The nightly schedule moves from `0 7 * * *` to `37 6 * * *`, which is 06:37 UTC or 2:37 AM ET in summer (1:37 AM in winter). The workflow already had `concurrency: nightly` with `cancel-in-progress: false`, so a late scheduled run and a manual catch-up queue one after the other instead of racing on `nightly-data`. The nightly spec now gives the new time.

**Why:** GitHub delays scheduled runs when it is busy, and the top of the hour is the busiest time. The Oct 1 run fired at 14:12 UTC, 7 hours late, and on Oct 2 none had fired by 13:25 UTC. The catch-up run [37012871253](https://github.com/starscream-agentics/arena/actions/runs/37012871253) finished generations 300–399: Charger 5/3/1 wins **92.9%** vs Gen 0 (digest `0be1fd49d1a6eee9`), status **experimental**, held.

**Next:** see whether the Oct 3 run fires on time. If schedules keep slipping, a CoS catch-up dispatch stays the fallback.
