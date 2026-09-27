# Starscream

An agentic game company building in public: a Rust engine, Tank Arena, and a devlog.

**Starscream Agentics** is run by agents. A Chief of Staff agent plans, dispatches worker agents, gates merges on CI, and reports to the founder, Nye Warburton (Creative Director). Every line of code comes from agents; Nye makes the calls on taste and approvals.

**Current mission — Tank Arena:** autonomous tank agents fighting in a bounded arena, running live in the browser, getting measurably better through self-play.

Status: **Phase 0 (scaffold)**. See [docs/STATE.md](docs/STATE.md).

## Layout
```
engine/        Rust library: deterministic 60 Hz sim core (also compiles to wasm)
engine-cli/    headless runner: N matches -> JSON (source of truth for CI)
games/tank/    Tank Arena rules, observations, actions, policies
web/           the Vercel site: viewer + devlog page
docs/          charter, state, decisions, role briefs, devlog, playbooks
.github/       CI and nightly workflows
```

## Run it
Requires Rust stable (pinned via `rust-toolchain.toml`).
```sh
cargo test --workspace
cargo run -p engine-cli -- --matches 10 --seed 42
# {"matches":10,"seed":42,"results":[]}
cargo build -p engine --target wasm32-unknown-unknown
```

## Docs
- [Charter](docs/CHARTER.md): how the company runs
- [State](docs/STATE.md): what's happening now
- [Decisions](docs/DECISIONS.md): architecture decision records
- [Roles](docs/roles/): worker briefs
- [Devlog](docs/devlog/): one entry per merged PR
- [Playbooks](docs/playbooks/): what each phase taught us

## Contributing
Issues and PRs from outside the company are welcome as information, but they are not merged or acted on without Nye's approval.

## License
[MIT](LICENSE)
