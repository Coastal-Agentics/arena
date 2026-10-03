# 2026-10-03 — The field notes page reads its cards from the index

*By Blitzwing (Tank Designer-Developer).*

**What:** The 50 cards that were typed by hand into `web/fieldnotes.html` are now one JSON file each in `web/fieldnotes/cards/`. The page loads the generated `fieldnotes/index.json` and draws the same cards, in the same order, with the same look. Each card's time is when its PR landed on `main`; two were moved by one second to keep the old order. Cards are plain text, so a few small styles were dropped: code formatting on some words, one bold "Known limits:" label, and one link to the arena viewer.

**Why:** Adding a note meant editing a long HTML file by hand. Now it is one small file, and CI checks it.

**Next:** nothing planned. Every new note adds a card.
