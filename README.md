# Coastal Agentics — Arena

**We train robots, with open tools, on the Georgia coast.** Coastal Agentics is an open source robotics company in Savannah, Georgia: we train agents in simulation and move them onto physical robots. Our tools and methods are open, reward functions are published, and every dataset records who made it.

Founded October 1, 2026 (formerly Starscream Agentics).

- Site: https://starscream-agentics.github.io/arena/ (the `starscream-agentics` org is the home for simulations; ADR-013)
- Field notes: [docs/fieldnotes/](docs/fieldnotes/) · [on the site](https://starscream-agentics.github.io/arena/fieldnotes.html)

This repo is the **arena**: a small deterministic Rust engine and the Tank Arena project. The browser viewer is live with Tank Arena rules-v1 (#18): 9-point loadouts, Charger/Kiter/Sniper policies, and a Customize tab with shareable links. The legacy built-in-bot viewer remains available. The company is run by agents: a Chief of Staff (Soundwave) plans, dispatches workers (Shockwave, Engine Lead; Blitzwing, Tank Designer-Developer), merges on green CI, and reports to the founder, Nye Warburton (Creative Director).

Status: **Phase 2 (Tank Arena)**. Rules-v1 is live (#18) with 9-point loadouts, Charger/Kiter/Sniper policies, and the Customize tab with shareable links. The Watch-tab fix is in (#20), and the viewer browser check runs in CI (#21). See [docs/STATE.md](docs/STATE.md).

## Layout
```
engine/        Rust sim core: deterministic 60 Hz, replays (also compiles to wasm); today it also
               holds the tank step rules, TankParams and observations/actions (ADR-009, ADR-014)
engine-cli/    headless runner: N matches -> JSON (source of truth for CI)
engine-wasm/   browser bindings for the viewer (wasm-bindgen), built into web/pkg
games/tank/    Tank Arena rules v1: loadouts, arena and spawns, scripted policies, and the
               placeholder bots Chaser and Wanderer used by engine-cli and the viewer's built-in-bot mode
web/           the GitHub Pages site: viewer + field notes (static; the only generated file is
               web/fieldnotes/index.json, built from web/fieldnotes/cards/)
docs/          charter, state, decisions, provenance card, role briefs, field notes, playbooks
.github/       CI, nightly and Pages workflows
```

## Run it
Requires Rust stable (pinned via `rust-toolchain.toml`).
```sh
cargo test --workspace
cargo run -p engine-cli -- --matches 10 --seed 42
cargo build -p engine --target wasm32-unknown-unknown
```

Preview the site locally. The field notes page reads `web/fieldnotes/index.json`, which is generated and not committed, so build it first:
```sh
node scripts/fieldnotes_index.mjs        # Node 20+; writes web/fieldnotes/index.json
python3 -m http.server -d web 8000       # open http://localhost:8000/fieldnotes.html
```

## Docs
- [Charter](docs/CHARTER.md): how the company runs
- [State](docs/STATE.md): what's happening now
- [Decisions](docs/DECISIONS.md): architecture decision records
- [Card](docs/CARD.md): provenance card for every shipped artifact
- [Engine](docs/engine/): how the engine, engine-cli, replays and the wasm viewer work
- [Roles](docs/roles/): worker briefs
- [Field notes](docs/fieldnotes/): one entry per merged PR
- [Playbooks](docs/playbooks/): what each phase taught us

## Contributing
Issues and PRs from outside the company are welcome as information, but they are not merged or acted on without Nye's approval.

## License
Code is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Nyborg character designs, art and logos are **not** open source: see [ASSETS-LICENSE.md](ASSETS-LICENSE.md). Names and logos: [TRADEMARKS.md](TRADEMARKS.md). Third-party components: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). How this was made: [AUTHORS.md](AUTHORS.md).

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. Sign off every commit; see [CONTRIBUTING.md](CONTRIBUTING.md).
