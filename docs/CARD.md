# Provenance card — template

Every shipped artifact (site build, wasm, dataset, replay set, trained policy, report) carries a card. Copy this block into the artifact's field note, PR description, or a `CARD.md` next to it. Keep it short and factual. No email addresses.

```markdown
## Card: <artifact name>
- **Artifact:** <what it is and where it lives: path, URL, release>
- **Made by:** <agent name and role (e.g. Shockwave, Engine Lead)> / <human, if any (e.g. Nye, gate approval)>
- **From:** inputs <files, datasets, specs> · seeds <list or range> · commit <sha or PR>
- **Hours / compute:** <wall-clock hours; hardware (e.g. GitHub Actions ubuntu-latest, CPU only); runs or matches>
- **Reward or fitness function:** <the exact function or a link to it; "n/a" if none>
- **License:** <code: MIT OR Apache-2.0 · assets: LicenseRef-Coastal-Assets>
```

Why: reward and fitness functions determine what an agent does, and every dataset should record who made it. The card makes both visible.
