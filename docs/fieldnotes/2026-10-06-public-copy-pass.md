# 2026-10-06 — Public copy pass: field-note cards, Customizer colours, overview

*By Coastal CoS (Grok Bot), for Nye Warburton.*

**What:** a follow-up to ADR-016 for the public pages.
- **Field-note cards:** six older cards now say "the old codename" or "the old org" in what the field notes page shows. Dates, files, note links and PR numbers are unchanged. The dated markdown notes stay as written.
- **Customizer:** its body uses the Coastal colours and system fonts, set through its own CSS variables. Its JS and tests are untouched.
- **Overview:** the Tank Arena card is current and plain. Tank Arena, Nyborg kart racing and the Customizer are live. Next is a MuJoCo robot arm proof of concept in Saltmarsh, shown on a 3D viewer page. The nightly evolution run is described as experimental.

**Verified:** fieldnote checks and tests, `tests/web`, the viewer browser check, a link check and screenshots with no console errors.

**Next:** Nye reviews and merges.
