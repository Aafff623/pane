// node --test tests/i18n-pairing.test.mjs — app i18n invariant lock.
//
// The three parked dictionaries (en/zh/ru) must carry identical key sets:
// t()'s fallback chain hides a missing key behind English, so a translation
// gap is invisible until a user sees a whole page regress to English (the
// ru One/New API block shipped half-English for exactly this reason).
// METRIC_KEYS values must exist in every dictionary for the same reason.
//
// Zero dependencies by design — it parses the TS source as text, so it runs
// on bare node in CI (same pattern as site/site.test.mjs).
import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const src = await readFile(new URL("../src/i18n.ts", import.meta.url), "utf8");

function dictKeys(name) {
  const start = src.indexOf(`const ${name}: Dict`);
  assert.ok(start >= 0, `dictionary ${name} not found`);
  const bounds = ["en", "zh", "ru"]
    .filter((n) => n !== name)
    .map((n) => src.indexOf(`const ${n}:`, start + 10))
    .concat([src.indexOf("const METRIC_KEYS")])
    .filter((i) => i > start);
  const block = src.slice(start, bounds.length ? Math.min(...bounds) : src.length);
  return new Set([...block.matchAll(/"([^"]+)":/g)].map((m) => m[1]));
}

test("i18n: en/zh/ru carry identical key sets", () => {
  const en = dictKeys("en");
  const zh = dictKeys("zh");
  const ru = dictKeys("ru");
  assert.deepEqual([...en].filter((k) => !zh.has(k)), [], "keys in en missing from zh");
  assert.deepEqual([...zh].filter((k) => !en.has(k)), [], "keys in zh missing from en");
  assert.deepEqual([...en].filter((k) => !ru.has(k)), [], "keys in en missing from ru");
  assert.deepEqual([...ru].filter((k) => !en.has(k)), [], "keys in ru missing from en");
});

test("i18n: every METRIC_KEYS target exists in all three dictionaries", () => {
  const start = src.indexOf("const METRIC_KEYS");
  const end = src.indexOf("\n};", start);
  const targets = [...src.slice(start, end).matchAll(/: "(metric\.[^"]+)"/g)].map((m) => m[1]);
  assert.ok(targets.length >= 10, `unexpected METRIC_KEYS size: ${targets.length}`);
  for (const dict of ["en", "zh", "ru"]) {
    const keys = dictKeys(dict);
    const missing = targets.filter((k) => !keys.has(k));
    assert.deepEqual(missing, [], `METRIC_KEYS targets missing from ${dict}`);
  }
});
