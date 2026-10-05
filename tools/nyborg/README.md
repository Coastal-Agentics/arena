# Nyborg drawing scripts

These are the scripts that drew the Nyborg mockups and wireframe in `docs/design/`. They are kept here as a record of how those images were made.

- `gen-r1.py`, `gen-r2.py` and `gen.py` drew revisions 1, 2 and 3 of `docs/design/nyborg-mockup.svg`. Revision 3 (`gen.py`) is the approved design.
- `wireframe.py` drew `docs/design/nyborg-library-wireframe.svg` from the revision 3 drawing.

The scripts are code under MIT OR Apache-2.0 (see the repository README). The images they produce, and the Nyborg character design itself, are covered by [ASSETS-LICENSE.md](../../ASSETS-LICENSE.md), not the code licenses.

They are copied unchanged from the design run, apart from the license header, so their output paths are absolute paths from the environment that ran them. Change `OUT` (and the source path in `wireframe.py`) before you run them.
