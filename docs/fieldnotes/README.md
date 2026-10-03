# Fieldnotes index

Fieldnote cards live in `web/fieldnotes/cards/`, one strict version-1 JSON file per note. Generate the Pages index with:

```sh
node scripts/fieldnotes_index.mjs
```

Use `node scripts/fieldnotes_index.mjs --check` to validate cards without writing. The generated `web/fieldnotes/index.json` is ignored because deployment creates it; cards are sorted newest first by the actual `published` timestamp, then by filename. Card objects reject unknown fields, and their `note` paths must point to files in the repository.

## Adding a card

1. Write the note: `docs/fieldnotes/YYYY-MM-DD-<slug>.md`.
2. Add its card: `web/fieldnotes/cards/YYYY-MM-DD-HHMM-<slug>.json`, where the date matches `date` and HHMM is the time in `published`. Required: `format` (1), `date`, `published` (ISO time with offset, e.g. `2026-10-03T12:10:00-04:00`), `title`, `author`, `body` (plain-text paragraphs, no HTML), `note` (the note's repo path). Optional: `next`, `pr`.
3. Run `node scripts/fieldnotes_index.mjs --check`. Don't edit `web/fieldnotes.html` or commit `index.json`; the page renders every card from the index, newest `published` first.
