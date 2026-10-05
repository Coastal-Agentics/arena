# Authors and agent authorship

**Copyright holder:** Nye Warburton. If a company is formed for the project and this work is assigned to it, the holder becomes that company. See NOTICE and the license files.

## People
- **Nye Warburton** ([@NyeGuy](https://github.com/NyeGuy)): founder, creative director and maintainer. Nye sets the specifications and gates, reviews the work, and decides what is merged and published.

## AI agents
Much of the code, documentation and art in this repository was produced by **AI software agents that Nye Warburton operates, on his behalf and under his direction**.
- The agents are tools. They are not legal authors and hold no rights.
- They work from the specifications, design gates and feedback that Nye sets.
- Nye's contributions are the direction, the design decisions, the selection among options, the edits and the approvals.

| Agent | Role |
|---|---|
| Soundwave | Chief of Staff: plans, dispatches, merges pull requests that pass CI and stay in scope |
| Shockwave | Engine Lead |
| Blitzwing | Tank Designer-Developer (games, Nyborg design) |

The agents run on Grok Bot. Each run is automated, and many runs start on a schedule. **A commit's timestamp records when an agent ran, not when a person was working.**

## How agent commits are marked
- **Until the `coastal-agentics-bot` GitHub App is in use** (it is not set up yet): agent commits are made under Nye's GitHub account (`NyeGuy`). Some show the author name "Nye"; others show a role name such as "Coastal CoS (Grok Bot)" or "Starscream Engine Lead (Grok Bot)" and carry that name's own `Signed-off-by`. The field notes (`docs/fieldnotes/` and `web/fieldnotes/cards/`) name the agent behind each merged pull request. Pull requests are opened and merged through the same account. History is left as it is. This file is the record of how to read it.
- **Once the GitHub App is in use:** agent commits are authored by `coastal-agentics-bot[bot]` and carry these trailers:
  - `Agent-Run: <agent name and role>, <run id or time>`
  - `On-Behalf-Of: Nye Warburton`
  - `Directed-By: Nye Warburton (<gate or request id>)`, when a specific instruction started the run
  - `Signed-off-by: Nye Warburton <7074155+NyeGuy@users.noreply.github.com>`, Nye's Developer Certificate of Origin sign-off. Nye authorizes it in writing for work he directs.
- **Outside contributors** sign off their own commits. See [CONTRIBUTING.md](CONTRIBUTING.md).
