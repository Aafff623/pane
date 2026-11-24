import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

// Regression lock for the board routing rules (owner, 2026-11-24):
//
//   1. A provider the user enabled must appear in some group.
//   2. There is exactly one built-in group ("default"); everything else is
//      user-created and deletable.
//   3. Enabled-but-signed-out providers show too — no category taxonomy may
//      swallow them.
//   4. A manual group tag survives a disable/enable cycle, and deleting the
//      group sends its members back to the default group.

const main = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");

function bodyOf(name) {
  const start = main.indexOf(`function ${name}(`);
  assert.ok(start > 0, `${name} should exist`);
  // Walk braces so nested blocks are included.
  let depth = 0;
  let i = main.indexOf("{", start);
  const from = i;
  for (; i < main.length; i++) {
    if (main[i] === "{") depth += 1;
    else if (main[i] === "}") {
      depth -= 1;
      if (depth === 0) return main.slice(from, i + 1);
    }
  }
  throw new Error(`${name} body not closed`);
}

test("one built-in group ships, and it is the default group", () => {
  assert.match(main, /const DEFAULT_GROUP = "default" as const;/);
  // The catalog taxonomy must not come back as a routing layer.
  assert.doesNotMatch(main, /const OVERVIEW_CATEGORIES = \["coding"/);
});

test("a disabled provider is dropped before any grouping decision", () => {
  const body = bodyOf("orderedSnapshots");
  assert.match(body, /isCardDisabled/, "disabled cards must be filtered out of the board list");
});

test("enabling never routes a provider into a hidden category", () => {
  // The overview and the board both render every visible card; neither may
  // filter by a catalog category any more.
  assert.doesNotMatch(main, /effectiveCategory\(providerFamily/);
  assert.doesNotMatch(main, /overviewCategory === /);
});

test("deleting a group releases its members to the default group", () => {
  const body = bodyOf("deleteCardGroup");
  assert.doesNotMatch(
    body,
    /cardGroupMemberCount\(id\) > 0\) return false/,
    "a group with members must still be deletable",
  );
  assert.match(body, /delete entry\.group/, "members must lose the stale tag");
});

test("a group tag lives in layout, independent of the enable list", () => {
  const body = bodyOf("setCardGroup");
  assert.match(body, /layout\.providers\[fam\]\.group = groupId/);
  // Disabling only touches config.disabled; it must not clear the tag.
  const disable = main.slice(
    main.indexOf("function handleCustomizeChange"),
    main.indexOf("function handleCustomizeChange") + 1600,
  );
  assert.doesNotMatch(disable, /\.group\s*=\s*undefined/, "disabling must keep the group tag");
  assert.doesNotMatch(disable, /delete .*\.group/, "disabling must keep the group tag");
});
