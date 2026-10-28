import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const i18n = readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");
const rust = readFileSync(new URL("../src-tauri/src/providers/codebuddy.rs", import.meta.url), "utf8");
const official = readFileSync(new URL("../src-tauri/src/providers/workbuddy_official_usage.rs", import.meta.url), "utf8");

test("WorkBuddy usage is a structured snapshot field, not a string", () => {
  assert.match(main, /interface WorkBuddyUsage\b/);
  assert.match(main, /workbuddy_usage\?: WorkBuddyUsage \| null/);
  assert.match(rust, /fn snapshot_usage\(&self\)/);
  assert.match(rust, /snapshot\.workbuddy_usage = official\.map/);
  assert.match(official, /pub struct WorkBuddyUsage\b/);
  assert.match(official, /pub by_day: Vec<WorkBuddyUsageDay>/);
  assert.match(official, /pub rows: Vec<WorkBuddyUsageRow>/);
});

test("WorkBuddy card expands to a daily trend and recent requests", () => {
  assert.match(main, /function workbuddyDetailsHtml/);
  assert.match(main, /<details class="wb-details">/);
  assert.match(main, /wb-chart/);
  assert.match(main, /wb-rows/);
  assert.match(main, /body \+= workbuddyDetailsHtml\(shown\)/);
});

test("WorkBuddy detail strings exist in all three languages", () => {
  for (const key of ["wb.history", "wb.daily", "wb.requests", "wb.partial", "wb.empty", "wb.credits"]) {
    const hits = i18n.match(new RegExp(`"${key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"`, "g"))?.length ?? 0;
    assert.equal(hits, 3, `${key} must exist in en/zh/ru`);
  }
});
