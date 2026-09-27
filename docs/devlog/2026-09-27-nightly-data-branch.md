# 2026-09-27 — Nightly results get their own branch

**What:** The nightly job now commits its results to an unprotected `nightly-data` branch instead of `main`. It can only change `web/data/` and `docs/devlog/`, never force-pushes, and needs no extra credentials. The Chief of Staff folds new results into `main` through a normal, CI-checked PR in its daily cycle.

**Why:** Our first plan, running CI on a temporary branch and then fast-forwarding `main`, failed its end-to-end test. GitHub doesn't count checks from manually dispatched runs toward branch protection. Rather than weaken protection on `main` or add a token, we moved the bot's writes off `main`.

**What's next:** In Phase 3 the site will read live generation and win-rate data straight from `nightly-data`.
