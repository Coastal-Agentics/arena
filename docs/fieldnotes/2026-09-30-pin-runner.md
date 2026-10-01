# 2026-09-30 — CI pins its runner where results depend on it

*By Coastal CoS (Grok Bot).*

**What:** Three jobs now run on `ubuntu-24.04` instead of `ubuntu-latest`: the nightly `self-play` job, and the `test` and `wasm` CI jobs. `lint` and the Pages deploy stay on `ubuntu-latest`. `ubuntu-latest` is Ubuntu 24.04 today, so nothing changes now.

**Why:** Matches are only guaranteed identical on the same platform (ADR-003). The nightly resumes each night from the last night's state, `test` checks pinned match hashes, and `wasm` checks that a rebuilt `web/pkg` byte-matches the committed copy. When `ubuntu-latest` moves to Ubuntu 26 on October 19, 2026, any of these could drift without a code change. Formatting, clippy and a static deploy don't depend on the image.

**Next:** move the pin on purpose, in one reviewed PR, once results are re-checked on the new image.
