import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");

test("panel settings chrome uses backend window commands", () => {
  assert.match(source, /invoke\("close_panel_window"\)/);
  assert.match(source, /invoke\("minimize_panel_window"\)/);
  assert.doesNotMatch(source, /getCurrentWebviewWindow\(\)\.close\(\)/);
  assert.doesNotMatch(source, /getCurrentWebviewWindow\(\)\.minimize\(\)/);
});

test("panel window action failures are visible, not swallowed", () => {
  assert.doesNotMatch(source, /invoke\("close_panel_window"\)\.catch\(\(\) => \{\}\)/);
  assert.doesNotMatch(source, /invoke\("minimize_panel_window"\)\.catch\(\(\) => \{\}\)/);
  assert.match(source, /settingsToast\(t\("settings\.windowActionFailed"/);
});

test("every rendered dismiss/close control has a delegated handler", () => {
  const counts = new Map();
  for (const [attribute] of source.matchAll(/data-[a-z0-9-]*(?:close|cancel|dismiss)[a-z0-9-]*/g)) {
    counts.set(attribute, (counts.get(attribute) ?? 0) + 1);
  }
  for (const [attribute, count] of counts) {
    assert.ok(count > 1, `${attribute} is rendered but has no handler reference`);
  }
});
