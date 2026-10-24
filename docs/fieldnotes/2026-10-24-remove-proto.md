# 2026-10-24 — Proto hair sheet retired

*By Blitzwing /nyborg.*

**What:** the leftover hair prototype under `web/proto/` is gone. Production hair has lived in `web/nyborg-hair.js` since milestone 5; the comparison sheet page and its Node test were only serving that unused proto.
- Deleted `web/proto/hair-sheet.html` and `web/proto/nyborg-hair.js`.
- Deleted `tests/web/hair-proto.test.mjs` (the production suite is `tests/web/hair.test.mjs`).
- Pointed `docs/design/nyborg-app.md` at the production module and the hair options PNG.

**Verified:** `node --test tests/web/hair.test.mjs tests/web/app.test.mjs` passes. Fieldnotes check passes. No linked page linked to `/arena/proto/`.

**Next:** a mobile pass on the Nyborg app for friends on phones.
