// Build the static fieldnotes index from one JSON card per note.
// Usage:
//   node scripts/fieldnotes_index.mjs         # write web/fieldnotes/index.json
//   node scripts/fieldnotes_index.mjs --check # validate cards without writing
// Cards are strict version-1 objects; the note path is relative to the repo root.

import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const CARD_NAME = /^(\d{4}-\d{2}-\d{2})-\d{4}-.+\.json$/;
const CARD_KEYS = ["format", "date", "published", "title", "author", "body", "next", "note", "pr"];
const REQUIRED_KEYS = ["format", "date", "published", "title", "author", "body", "note"];

function errorFor(file, message) {
  throw new Error(`${file}: ${message}`);
}

function stringField(card, key, file) {
  if (typeof card[key] !== "string") errorFor(file, `${key} must be a string`);
  return card[key];
}

function validateCard(repoRoot, cardsDir, filename) {
  const match = CARD_NAME.exec(filename);
  if (!match) errorFor(filename, "filename must match YYYY-MM-DD-HHMM-slug.json");

  const path = resolve(cardsDir, filename);
  let card;
  try {
    card = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    errorFor(filename, `invalid JSON (${error.message})`);
  }
  if (card === null || typeof card !== "object" || Array.isArray(card)) {
    errorFor(filename, "card must be a JSON object");
  }

  for (const key of Object.keys(card)) {
    if (!CARD_KEYS.includes(key)) errorFor(filename, `unknown field ${key}`);
  }
  for (const key of REQUIRED_KEYS) {
    if (!Object.hasOwn(card, key)) errorFor(filename, `missing required field ${key}`);
  }
  if (typeof card.format !== "number") errorFor(filename, "format must be a number");
  if (card.format !== 1) errorFor(filename, "unknown format (expected 1)");

  const date = stringField(card, "date", filename);
  if (date !== match[1]) errorFor(filename, `date must match filename date ${match[1]}`);
  const published = stringField(card, "published", filename);
  const publishedMs = Date.parse(published);
  if (!Number.isFinite(publishedMs)) errorFor(filename, "published must be a valid timestamp");
  stringField(card, "title", filename);
  stringField(card, "author", filename);
  stringField(card, "note", filename);

  if (!Array.isArray(card.body) || card.body.length === 0 || card.body.some((paragraph) => typeof paragraph !== "string")) {
    errorFor(filename, "body must be a non-empty array of strings");
  }
  if (Object.hasOwn(card, "next") && typeof card.next !== "string") errorFor(filename, "next must be a string");
  if (Object.hasOwn(card, "pr") && (!Number.isInteger(card.pr) || card.pr <= 0)) {
    errorFor(filename, "pr must be a positive integer");
  }

  const notePath = card.note;
  if (isAbsolute(notePath)) errorFor(filename, "note must be relative to the repo root");
  const resolvedNote = resolve(repoRoot, notePath);
  const fromRoot = relative(repoRoot, resolvedNote);
  if (fromRoot.startsWith("..") || isAbsolute(fromRoot) || !existsSync(resolvedNote) || !statSync(resolvedNote).isFile()) {
    errorFor(filename, `note path does not exist relative to the repo root: ${notePath}`);
  }

  const ordered = {};
  for (const key of CARD_KEYS) if (Object.hasOwn(card, key)) ordered[key] = card[key];
  return { filename, publishedMs, card: ordered };
}

export function buildIndex(repoRoot) {
  const root = resolve(repoRoot);
  const cardsDir = resolve(root, "web/fieldnotes/cards");
  if (!existsSync(cardsDir)) return { format: 1, cards: [] };
  if (!statSync(cardsDir).isDirectory()) errorFor("web/fieldnotes/cards", "cards path must be a directory");

  const entries = readdirSync(cardsDir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name));
  const validated = [];
  for (const entry of entries) {
    if (!entry.isFile()) errorFor(entry.name, "card entry must be a regular file");
    validated.push(validateCard(root, cardsDir, entry.name));
  }
  validated.sort((a, b) => b.publishedMs - a.publishedMs || (a.filename < b.filename ? -1 : a.filename > b.filename ? 1 : 0));
  return { format: 1, cards: validated.map(({ card }) => card) };
}

export function writeIndex(repoRoot, check = false) {
  const index = buildIndex(repoRoot);
  if (!check) {
    const output = resolve(repoRoot, "web/fieldnotes/index.json");
    mkdirSync(dirname(output), { recursive: true });
    writeFileSync(output, `${JSON.stringify(index)}\n`);
  }
  return index;
}

export function main(argv = process.argv.slice(2), repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")) {
  if (argv.length > 1 || (argv.length === 1 && argv[0] !== "--check")) {
    console.error("usage: node scripts/fieldnotes_index.mjs [--check]");
    return 2;
  }
  try {
    const index = writeIndex(repoRoot, argv[0] === "--check");
    if (argv[0] === "--check") console.log(`fieldnotes_index: validated ${index.cards.length} card(s)`);
    return 0;
  } catch (error) {
    console.error(`fieldnotes_index: ${error.message}`);
    return 1;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) process.exitCode = main();
