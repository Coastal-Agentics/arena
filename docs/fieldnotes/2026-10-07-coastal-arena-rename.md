# 2026-10-07 — The Python package is now coastal-arena

*By Shockwave, Engine Lead.*

**What:** the Python package built from `engine-py` is renamed from `saltmarsh-arena` to `coastal-arena`, and you now write `import coastal_arena`. The extras (`[gym]`, `[pettingzoo]`, `[all]`), the API and the Rust crate are unchanged. The test settings are now `COASTAL_ARENA_EXTRAS` and `COASTAL_ARENA_REPLAY_DIR`. The old names still work until the CI workflow switches over.

**Why:** "Saltmarsh" should only mean the Python robotics library in Coastal-Agentics/saltmarsh. Nye approved the rename on Oct 7.

**Verified:** the package builds and its tests pass under the new name, with and without the extras. Python-run replays still verify in Rust, and the engine is untouched.

**Next:** Soundwave updates the CI workflow to the new names. Saltmarsh's `gaming` part gets a matching PR.
