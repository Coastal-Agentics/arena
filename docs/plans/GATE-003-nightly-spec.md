# GATE-003 M1 — nightly workflow change (spec for the workflow owner)

*By Blitzwing, Tank Designer-Developer, 2026-09-30. For Soundwave (CoS), who owns `.github/workflows/`. This spec describes the change; Blitzwing does not edit `.github/`. It covers the GATE-003 plan's M2 "nightly runs it" step for the M1 loop (`games/tank/src/evolve/`, `games/tank/examples/evolve.rs`).*

## What changes

This is one edit to `.github/workflows/nightly.yml`. The step `Self-play evolution (placeholder)` is replaced by the four steps below. A toolchain step is added before them, because the job has none today. Everything else stays as it is: the triggers, `permissions`, `concurrency`, the `nightly-data` switch-and-merge step, the probe step, the allowed-paths commit step and the never-force push (ADR-007).

| Setting | Value |
|---|---|
| Schedule | `cron: "37 6 * * *"` (moved from `"0 7 * * *"` on 2026-10-02 because GitHub delayed the top-of-the-hour schedule by up to 7 hours). That is 06:37 UTC, which is 2:37 AM ET in summer (EDT) and 1:37 AM ET in winter (EST). `workflow_dispatch` is unchanged, and `concurrency: nightly` with `cancel-in-progress: false` queues a late scheduled run behind a manual one. |
| Runner | `ubuntu-24.04`, pinned in PR #29 (public repo: 4 CPUs, 16 GB). Not `ubuntu-latest`: results are byte-identical only on the same platform (ADR-003). |
| Job timeout | `timeout-minutes: 60`, unchanged. |
| Training cap | `--minutes 30` and `--generations 100` per night. The loop checks the clock before each generation and stops early if the cap is reached, keeping every finished generation. |
| Expected runtime | About 4 minutes on 4 cores. Seed 1's first 100 generations took 147 s on 8 threads (14.5 CPU-minutes), plus about 2 s for the held-out check. Build time comes on top. |

**Runner pin (CoS, 2026-09-30):** the `self-play` job now runs on `ubuntu-24.04` instead of `ubuntu-latest`, so results stay byte-identical when `ubuntu-latest` moves to Ubuntu 26 on October 19, 2026 (ADR-003: same platform only).

## Steps (insert after "Switch to data branch … and merge main")

```yaml
      - name: Install toolchain (from rust-toolchain.toml)
        run: rustup toolchain install && rustup show

      - uses: Swatinem/rust-cache@v2

      - name: Self-play evolution (GATE-003 M1)
        run: |
          set -euo pipefail
          OUT=web/data/evolution
          mkdir -p "$OUT"
          RESUME=""
          if [ -f "$OUT/state.json" ]; then RESUME="--resume"; fi
          cargo run -p tank --release --locked --example evolve -- run \
            --out "$OUT" --seed 1 --generations 100 --minutes 30 \
            --threads "$(nproc)" --commit "$(git rev-parse HEAD)" $RESUME

      - name: Re-verify the champion by hash
        run: cargo run -p tank --release --locked --example evolve -- verify --out web/data/evolution --threads "$(nproc)"

      - name: Size caps and promotion status
        run: |
          set -euo pipefail
          OUT=web/data/evolution
          total=$(cat "$OUT"/*.json | wc -c)
          if [ "$total" -gt 5000000 ]; then echo "::error::$OUT is $total B (cap 5 MB)"; exit 1; fi
          git add -A web/data
          changed=$(git diff --cached --name-only --diff-filter=d -- web/data | xargs -r cat | wc -c)
          if [ "$changed" -gt 200000 ]; then echo "::error::nightly change is $changed B (cap 200 KB)"; exit 1; fi
          status=$(jq -r .held_out.status "$OUT/champion.json")
          label=$(jq -r .label "$OUT/champion.json")
          gen=$(jq -r .generation "$OUT/champion.json")
          rate=$(jq -r .held_out.win_rate "$OUT/champion.json")
          echo "Gen $gen champion $label: $rate vs Gen 0 on 1,000 held-out seeds, status $status" >> "$GITHUB_STEP_SUMMARY"
          if [ "$status" = "experimental" ]; then
            echo "::warning::Gen $gen ($label) wins $rate vs the scripted field (> 70%): HELD from promotion, labeled experimental. CoS: tell Nye."
          fi
```

The existing `Commit results (allowed paths only)` step then commits `web/data/evolution/*.json`; that path is already allowed. If any of the new steps fails, the job stops before the commit and the push, so nothing is published.

## `nightly-data` layout

All files live under `web/data/evolution/` on `nightly-data`. The CLI writes them without timestamps, so the same inputs give the same bytes.

| File | What it holds | Cap (enforced by) |
|---|---|---|
| `state.json` | GA settings, next generation, the 32-candidate population and the hall of fame. `--resume` continues from it. | 64 KB (CLI); about 5 KB today |
| `champion.json` | The latest champion: genome, training score, held-out W/D/L vs Gen 0, digest, status, fitness text, config and commit. | 16 KB (CLI); about 1.7 KB |
| `history.json` | One line per generation (best, mean, champion label, vs-scripted rate, behavior mix). Entries beyond the latest 2,000 are thinned to every 10th generation. | part of the 5 MB total; about 100 B per generation |
| `genomes.json` | The champion of every 10th generation, for the viewer's slider (M2). | part of the 5 MB total; about 330 B per entry |

- **Total:** at most 5 MB (checked by the CLI and the workflow). Each nightly change is at most 200 KB (checked by the workflow).
- **No replay files:** the viewer re-simulates from (genome, loadout, seed).
- **Lineage:** the run seed is fixed at `--seed 1`. Nightly runs resume from `state.json`, so night *k* equals an uninterrupted run of 100·*k* generations (see determinism below).
- **Starting a new lineage:** a new seed, or changed gene tables (which make `state.json` refuse to load, and the job fails loudly), starts a new lineage. The new lineage reuses `web/data/evolution/`. The old lineage's files move on `nightly-data` to `web/data/evolution/archive/<name>/`, with a README giving its seed, generations and last champion, and the next night starts at Gen 0 because there is no `state.json`. The first archive is `archive/pre-dodge-2026-10-02/` (Gen 0–399, from before the dodge reflex; PR #38). Archiving is a move pushed without force. Nothing is deleted from `nightly-data` without Nye: deleting or force-pushing it is a gate (ADR-007).

## Promotion rule and the 70% hold

`champion.json` → `held_out.status` is computed from the win rate vs Gen 0 (the scripted defaults at 3/3/3) over 1,000 held-out seeds × 3 opponents × 2 sides:

| Status | Win rate vs Gen 0 | What happens |
|---|---|---|
| `below-bar` | < 65% | Published on `nightly-data` only. Not eligible. |
| `promotable` | 65–70% | Eligible. It becomes the viewer's default "Gen N" only through a reviewed PR (Blitzwing + CoS) with a `docs/CARD.md` provenance card. The nightly job never promotes. |
| `experimental` | > 70% | **Held from promotion and labeled experimental.** The job adds a warning and a line to the run summary, and the **CoS tells Nye** in the next work cycle. |

The CoS still folds `nightly-data` into `main` by a normal PR (ADR-007). Champion fold-ins reach `main` only that way, as a `nightly-data` → `main` PR that CI checks. Evolution files are never copied onto a branch of `main`. Copied files have no shared history with `nightly-data`, so git sees them as separate edits. On 2026-10-02 the copy promoted in #34 and #35 gave modify/delete conflicts with `nightly-data` and broke the nightly's merge of `main` (PR #38). The files carry their status, so an experimental champion stays labeled wherever it lands.

## Determinism checks (already run, 2026-09-30)

- The same seed gives byte-identical `state.json`, `champion.json`, `history.json` and `genomes.json` on 8 threads and on 4 threads.
- 50 generations followed by a `--resume` of 50 more gives the same files as one 100-generation run.
- `verify` replays all 6,000 held-out matches and compares W/D/L and the FNV-1a digest over every match's (opponent, seed, side, winner, ticks, state hash).
- ADR-003 promises determinism on the same platform, so the nightly verifies on the runner it trained on.

## Known result to expect on the first night

Seed 1's Gen 99 champion is `charger-5-3-1`. It wins **89.6%** vs Gen 0 (5,378 of 6,000; digest `3a4d9fd648b36362`), so it will be **`experimental`** and held. The main reason is that evolution switches on shell dodging (`dodge_horizon` > 0), which every shipped policy has off. The CoS should expect to tell Nye on night one.
