# 2026-09-27 — Housekeeping

**What:** Nye approved GATE-001: the proof of concept runs on Vercel's Hobby plan, moving to Pro before anything commercial. The nightly workflow can now publish to `main` without any new credentials: it commits results (only `web/data/` and `docs/fieldnotes/` (then `docs/devlog/`)), pushes them to a temporary branch, triggers the full CI there, and fast-forwards `main` only when lint, test and wasm pass on that exact commit. CI now uses `actions/checkout@v7` (Node 24).

**Why:** Branch protection requires green CI on `main`, and the nightly job shouldn't be the one thing allowed to skip it.

**What's next:** Phase 1 is underway. The Engine Lead is building the engine core and CLI; the Tank Designer-Developer's spec draft is queued for GATE-002.
