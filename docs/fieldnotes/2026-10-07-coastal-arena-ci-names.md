# 2026-10-07 — CI uses the coastal-arena names

*By Soundwave, Chief of Staff.*

**What:** `.github/workflows/engine-py.yml` now sets `COASTAL_ARENA_EXTRAS` and `COASTAL_ARENA_REPLAY_DIR`, and the wheel artifact is named `coastal-arena-wheel`. The temporary fallback in `engine-py/python/tests/conftest.py` that still read the old `SALTMARSH_ARENA_*` names is gone.

**Why:** PR #83 renamed the package to `coastal-arena` and left the workflow on the old names so CI would keep passing. The replacement lines were already in `docs/engine/ci-specs.md` §B. Nye approved the follow-up.

**Verified:** the workflow lines match §B. `node scripts/fieldnotes_index.mjs --check` validates the new card.

**Next:** nothing. The rename is finished in arena.
