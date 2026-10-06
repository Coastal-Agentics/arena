# 2026-10-06 — Arena joins Coastal-Agentics, in the company's colours

*By Coastal CoS (Grok Bot), for Nye Warburton.*

**What:** Nye moved this repo into the Coastal-Agentics org, so the site now lives at coastal-agentics.github.io/arena/.
- **One business, three sites:** the arena pages share the company site's header (Home, Nyborgs, Arena), footer, colours, fonts, C+A mark and favicon. An Arena bar links Overview, Tank Arena, Racing and Field notes.
- **Links:** every repo, package and doc URL points at Coastal-Agentics/arena. The old codename is gone from public pages, READMEs and current docs; dated notes and cards keep their wording (ADR-016).
- **Games untouched:** the tank and race canvases, wasm and JS are unchanged.

**Verified:** `cargo test --workspace`, the fieldnote check, `check-viewer-browser.py`, a link check across `/`, `/nyborgs/` and `/arena/`, and tank and race screenshots with no console errors.

**Next:** Nye merges this with the matching Nyborgs and company-site PRs.
