# Coastal Agentics — Arena

**We train robots, with open tools, on the Georgia coast.** Coastal Agentics is a consultancy and open source maintainer in Savannah, Georgia: we train agents in simulation and move them onto physical robots. Our tools and methods are open, reward functions are published, and every dataset records who made it.

Founded October 1, 2026 (formerly Starscream Agentics).

- Site: https://starscream-agentics.github.io/arena/ (the `starscream-agentics` org is the home for simulations; ADR-013)
- Field notes: [docs/fieldnotes/](docs/fieldnotes/) · [on the site](https://starscream-agentics.github.io/arena/fieldnotes.html)

This repo is the **arena**: a small deterministic Rust engine and the Tank Arena project. The built-in-bot viewer is live in the browser; the Tank Arena rules and policies await GATE-002 before implementation. The company is run by agents: a Chief of Staff (Soundwave) plans, dispatches workers (Shockwave, Engine Lead; Blitzwing, Tank Designer-Developer), merges on green CI, and reports to the founder, Nye Warburton (Creative Director).

Status: **Phase 2 (Tank Arena)**. The viewer is live at [arena.html](https://starscream-agentics.github.io/arena/arena.html); Tank Arena rules remain gated. See [docs/STATE.md](docs/STATE.md).

## Layout
```
engine/        Rust sim core: deterministic 60 Hz, replays (also compiles to wasm); today it also
               holds the tank rules, TankParams, observations/actions and placeholder bots (ADR-009)
engine-cli/    headless runner: N matches -> JSON (source of truth for CI)
engine-wasm/   browser bindings for the viewer (wasm-bindgen), built into web/pkg
games/tank/    stub for now; Tank Arena rules, observations, actions and policies move here later (ADR-009)
web/           the GitHub Pages site: viewer + field notes (static, no build step)
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
[MIT](LICENSE)
