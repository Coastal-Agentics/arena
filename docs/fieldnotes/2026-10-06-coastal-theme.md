# 2026-10-06 — Arena joins Coastal-Agentics, in the company's colours

*By Coastal CoS (Grok Bot), for Nye Warburton.*

**What:** Nye moved this repo into the Coastal-Agentics org, so the site now lives at coastal-agentics.github.io/arena/.
- **One business, three sites:** every arena page, the new Customizer (#73) included, shares the company site's header (Home, Nyborgs, Arena), footer, colours, logo and favicon. An Arena bar links Overview, Tank Arena, Racing, Customizer and Field notes.
- **Links:** repo, package and doc URLs point at Coastal-Agentics/arena. The old codename is gone from public pages and current docs; dated notes keep their wording (ADR-016).
- **CI:** the `wasm` job now runs the web module tests in `tests/web/`.
- **Games untouched:** the canvases, wasm and game JS are unchanged.

**Verified:** cargo tests, viewer and parity checks, web tests, a three-site link check, and screenshots with no console errors.

**Next:** Nye merges this with the matching Nyborgs and site PRs.
