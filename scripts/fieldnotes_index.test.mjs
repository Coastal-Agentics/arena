import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { buildIndex, main, writeIndex } from "./fieldnotes_index.mjs";

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "fieldnotes-index-"));
  mkdirSync(join(root, "web/fieldnotes/cards"), { recursive: true });
  mkdirSync(join(root, "docs/fieldnotes"), { recursive: true });
  writeFileSync(join(root, "docs/fieldnotes/note.md"), "note\n");
  return root;
}

function card(overrides = {}) {
  return {
    format: 1,
    date: "2026-10-03",
    published: "2026-10-03T11:55:00-04:00",
    title: "A note",
    author: "Author",
    body: ["Paragraph"],
    note: "docs/fieldnotes/note.md",
    ...overrides,
  };
}

function put(root, filename, value) {
  const path = join(root, "web/fieldnotes/cards", filename);
  writeFileSync(path, typeof value === "string" ? value : JSON.stringify(value));
}

function fails(root, expected, filename = "2026-10-03-1155-card.json", value = card()) {
  put(root, filename, value);
  assert.throws(() => buildIndex(root), new RegExp(`${filename}: ${expected}`));
  rmSync(join(root, "web/fieldnotes/cards", filename));
}

test("generates stable newest-first order and preserves optional fields", () => {
  const root = fixture();
  put(root, "2026-10-03-1155-zulu.json", card({ published: "2026-10-03T15:55:00Z", pr: 7, next: "another" }));
  put(root, "2026-10-03-1155-alpha.json", card({ published: "2026-10-03T11:55:00-04:00" }));
  put(root, "2026-10-03-1154-old.json", card({ published: "2026-10-03T11:54:00-04:00" }));
  const index = writeIndex(root);
  assert.deepEqual(index.cards.map((entry) => entry.title), ["A note", "A note", "A note"]);
  assert.equal(index.cards[1].next, "another");
  assert.equal(readFileSync(join(root, "web/fieldnotes/index.json"), "utf8"), `${JSON.stringify(index)}\n`);
  assert.deepEqual(Object.keys(index.cards[1]), ["format", "date", "published", "title", "author", "body", "next", "note", "pr"]);
  rmSync(root, { recursive: true });
});

test("missing cards directory produces an empty index", () => {
  const root = fixture();
  rmSync(join(root, "web/fieldnotes/cards"), { recursive: true });
  assert.deepEqual(buildIndex(root), { format: 1, cards: [] });
  rmSync(root, { recursive: true });
});

test("check mode validates without writing", () => {
  const root = fixture();
  assert.equal(main(["--check"], root), 0);
  assert.throws(() => readFileSync(join(root, "web/fieldnotes/index.json")));
  rmSync(root, { recursive: true });
});

test("rejects every missing required field", () => {
  for (const key of ["format", "date", "published", "title", "author", "body", "note"]) {
    const root = fixture();
    const value = card();
    delete value[key];
    fails(root, `missing required field ${key}`, "2026-10-03-1155-card.json", value);
    rmSync(root, { recursive: true });
  }
});

test("rejects wrong required field types", () => {
  for (const [key, value] of Object.entries({ format: "1", date: 1, published: 1, title: 1, author: 1, body: "paragraphs", note: 1 })) {
    const root = fixture();
    fails(root, `${key} must be`, "2026-10-03-1155-card.json", card({ [key]: value }));
    rmSync(root, { recursive: true });
  }
});

test("rejects unknown format and extra fields", () => {
  let root = fixture();
  fails(root, "unknown format", "2026-10-03-1155-card.json", card({ format: 2 }));
  rmSync(root, { recursive: true });
  root = fixture();
  fails(root, "unknown field extra", "2026-10-03-1155-card.json", card({ extra: "nope" }));
  rmSync(root, { recursive: true });
});

test("rejects invalid JSON", () => {
  const root = fixture();
  fails(root, "invalid JSON", "2026-10-03-1155-card.json", "{");
  rmSync(root, { recursive: true });
});

test("rejects filename and date mismatches", () => {
  let root = fixture();
  fails(root, "filename must match", "not-a-card.json");
  rmSync(root, { recursive: true });
  root = fixture();
  fails(root, "date must match filename date 2026-10-04", "2026-10-04-1155-card.json", card());
  rmSync(root, { recursive: true });
});

test("rejects missing note and invalid optional fields", () => {
  let root = fixture();
  fails(root, "note path does not exist", "2026-10-03-1155-card.json", card({ note: "docs/fieldnotes/missing.md" }));
  rmSync(root, { recursive: true });
  root = fixture();
  fails(root, "next must be", "2026-10-03-1155-card.json", card({ next: 3 }));
  rmSync(root, { recursive: true });
  root = fixture();
  fails(root, "pr must be", "2026-10-03-1155-card.json", card({ pr: 0 }));
  rmSync(root, { recursive: true });
});
