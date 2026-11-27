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

// Regression lock for the upgrade board (owner, 2026-11-26): a version that
// adds catalog providers must not fill the board with cards this machine
// cannot back. Two rules keep that honest.

test("a bare hint card is never exempt from the fold", () => {
  const body = bodyOf("isFoldableHintCard");
  assert.doesNotMatch(
    body,
    /STARTER_FAMILIES\.has/,
    "no family may stay permanently visible with no credential",
  );
  assert.match(body, /isBareHintCard/, "only credential-less cards fold");
});

test("the upgrade pick checks only what the probe detected", () => {
  const body = bodyOf("runProviderPicker");
  assert.match(body, /probed\.has\(id\)/, "a family the probe never answered for keeps its switch");
  assert.match(
    body,
    /checked: upgradeOnly \? false :/,
    "the starter suggestion must not be forced on during an upgrade",
  );
  assert.match(
    body,
    /checked: upgradeOnly \? !config\.disabled\.includes\(id\) :/,
    "an upgrade may retire empty cards but must never reopen a provider the user switched off",
  );
});

test("the upgrade pick is gated by the detection architecture, not the app version", () => {
  const body = bodyOf("queueUpgradePick");
  assert.match(
    body,
    /config\.setupPickedVersion === DETECTION_ARCH_VERSION/,
    "a plain version bump must never re-ask",
  );
  assert.doesNotMatch(
    body,
    /setupPickedVersion === appVersion/,
    "releasing a card must not re-prompt the detection picker",
  );
});

test("the picker's status chips filter the list", () => {
  const start = main.indexOf("function appProviderPicker(");
  const picker = main.slice(start, main.indexOf("function runProviderPicker("));
  assert.match(picker, /data-picker-filter/, "each health class needs a chip");
  assert.match(picker, /row\.hidden = filter !== null/, "a chip must narrow the visible rows");
  assert.match(
    picker,
    /filter = filter === next \? null : next/,
    "clicking the active chip again must clear the filter",
  );
  // Filtering is a view: a tick on a hidden row is still a tick.
  assert.match(
    picker,
    /Read every box, including rows a filter currently hides/,
    "confirming must read hidden rows too",
  );
  assert.match(picker, /picker-filter-count/, "each chip states how many rows it covers");
});

// Regression lock for the owner's overlap report (2026-11-26 screenshot): a
// 460px dialog squeezed the provider name into the status text.
test("the picker row keeps the name and the status in separate columns", () => {
  const css = readFileSync(new URL("../src/styles.css", import.meta.url), "utf8");
  const row = css.slice(css.indexOf(".picker-row {"), css.indexOf(".picker-row:hover"));
  assert.match(row, /display: grid/, "flex let the name overflow under the note");
  // Five tracks for five direct children (checkbox, dot, icon, name, status).
  // A missing track wraps the status onto its own line — measured live at
  // 640px, the status column landed under the name.
  assert.match(
    row,
    /grid-template-columns:\s*auto auto auto minmax\(0, 1fr\) auto/,
    "one track per direct child, or the status wraps",
  );
  const name = css.slice(css.indexOf(".picker-name {"), css.indexOf(".picker-source {"));
  assert.match(name, /text-overflow: ellipsis/, "a long name must truncate, not overflow");
  // `#confirm-box` sets width: min(88%, 300px) and an ID beats a class, so a
  // bare `.picker-box` selector silently stayed 300px wide — that is what
  // truncated names to "Qoder …" in the owner's screenshot.
  assert.match(
    css,
    /#confirm-box\.picker-box\s*\{[^}]*max-width:\s*640px/,
    "the picker width rule must outrank #confirm-box, or the dialog stays 300px",
  );
});

test("the picker shows each row's health and credential source", () => {
  // appProviderPicker returns a Promise, so bodyOf's brace walk stops at the
  // arrow body. Slice to the next top-level function instead.
  const start = main.indexOf("function appProviderPicker(");
  const picker = main.slice(start, main.indexOf("function runProviderPicker("));
  assert.match(picker, /picker-dot/, "each row needs the health dot");
  assert.match(picker, /is-\$\{r\.health\}/, "the dot gets its colour from the row health");
  assert.match(picker, /displayProbeSource/, "the raw probe source must be turned into words");
  // The classifier must map the three states the user asked for, and a
  // credential-less family must read as "nothing here", not as a failure.
  const health = bodyOf("pickerHealth");
  assert.match(health, /return "ok"/);
  assert.match(health, /return "warn"/);
  assert.match(health, /return "off"/);
  assert.match(health, /no_credentials"\) return "off"/);
});

test("dismissing the upgrade pick leaves the board alone", () => {
  const body = bodyOf("runProviderPicker");
  const dismiss = body.slice(body.indexOf("if (upgradeOnly && picked === null)"));
  assert.ok(dismiss.length > 0, "the upgrade pass needs an explicit dismiss branch");
  assert.match(
    dismiss.slice(0, dismiss.indexOf("const chosen")),
    /return;/,
    "a dismissed upgrade must not fall through to the default picks",
  );
});
