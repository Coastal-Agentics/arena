# 2026-10-11 — One editor: the Customizer moves into the app

*By Blitzwing /nyborg.*

**What:** milestone 4 of `docs/design/nyborg-app.md`. Your Nyborg is edited inside the app now, at `#/nyborg/<id>`.
- **Edit:** a big preview, the name, and the look (hair style, colour, strands for Classic yarn, accessories) on the left. On the right, **one build tab per arena**: Racing uses the schema-driven steppers and pips from the engine's catalog; the **Tank Arena keeps the loadout triangle** (drag or tap and the engine snaps to one of the 19 nine-point loadouts; arrow keys move a point), with presets and a readout. Builds save the moment the engine accepts them, never before. Below: **Race Pip ▶ / Tank Pip ▶**, Duplicate and Delete (with Undo for 10 s).
- **Getting there:** home's **Edit look & builds**, and **Tweak Pip** on the finish card. The old **`customizer.html` forwards** to your selected Nyborg's editor, with the same Nyborgs and storage keys, so nothing is lost. The viewer's tank tabs gain **✎ Edit your Nyborg in the app**.
- **Backup** (`#/backup`), moved up from milestone 6 so the redirect loses nothing: download all your Nyborgs or one, import a file (if a Nyborg is already here: skip, keep both or replace), and see any unreadable entries (download raw or remove).
- **Under the hood:** the arena plug-in contract gets an optional `editor(engine, build, onChange)`. Tank's is the triangle, and games without one get `app/build-editor.js` (the Customizer's build tab, moved). `customizer.js` is retired; the library, link and build-tab modules it used, and their tests, are unchanged.

**Verified:** in a headless browser, `customizer.html` lands on the selected Nyborg's editor. Rename and style changes save. An over-budget racing build (11/9) is refused, then saved at 9/9. A triangle tap snaps to 5/2/2 and ArrowRight to 4/2/3, Scout + sniper saves, and match setup shows the new build. Delete then Undo works. Export downloads both Nyborgs; re-import offers keep both / skip. The viewer link is in place, and the editor at 390 px has no sideways scroll. No console errors. Node tests: 30 pass, including the 19 triangle loadouts as engine-valid 9-point builds, the snap always landing on one, and import clash handling. `cargo test -q --workspace`, `check-viewer.mjs`, `check-parity.mjs` and the fieldnotes check pass.

**Next:** saving a Nyborg from someone's match link (milestone 6).
