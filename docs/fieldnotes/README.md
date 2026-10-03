# Fieldnotes index

Fieldnote cards live in `web/fieldnotes/cards/`, one strict version-1 JSON file per note. Generate the Pages index with:

```sh
node scripts/fieldnotes_index.mjs
```

Use `node scripts/fieldnotes_index.mjs --check` to validate cards without writing. The generated `web/fieldnotes/index.json` is ignored because deployment creates it; cards are sorted newest first by the actual `published` timestamp, then by filename. Card objects reject unknown fields, and their `note` paths must point to files in the repository.
