import test from "node:test";
import assert from "node:assert/strict";
import {
  canonicalBinding,
  effectiveBindings,
  eventBinding,
  filterShortcutCommands,
  findBindingConflict,
  isImeNoise,
  matchesBinding,
  migrateLegacyShortcuts,
  recordBinding,
  reservedBinding,
  takeoverBinding,
} from "../src/shortcutKernel.ts";

const key = (props = {}) => ({
  isComposing: false,
  keyCode: 0,
  repeat: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  key: "",
  code: "",
  ...props,
});

test("canonical form orders modifiers and resolves CmdOrCtrl per platform", () => {
  assert.equal(canonicalBinding("shift+ctrl+m", "windows"), "CTRL+SHIFT+M");
  assert.equal(canonicalBinding("CmdOrCtrl+M", "windows"), "CTRL+M");
  assert.equal(canonicalBinding("CmdOrCtrl+M", "mac"), "META+M");
  assert.equal(canonicalBinding("Cmd+M", "windows"), "META+M");
  assert.equal(canonicalBinding("", "windows"), null);
  assert.equal(canonicalBinding("Hyper+M", "windows"), null);
});

test("matching uses event.code so non-Latin layouts still hit the physical key", () => {
  assert.equal(
    matchesBinding(key({ ctrlKey: true, code: "KeyR", key: "й" }), "Ctrl+R", "windows"),
    true,
  );
  assert.equal(
    matchesBinding(key({ ctrlKey: true, code: "KeyR", key: "r" }), "CmdOrCtrl+R", "mac"),
    false,
  );
  assert.equal(
    matchesBinding(key({ metaKey: true, code: "KeyR", key: "r" }), "CmdOrCtrl+R", "mac"),
    true,
  );
  assert.equal(isImeNoise(key({ isComposing: true })), true);
  assert.equal(isImeNoise(key({ keyCode: 229 })), true);
  assert.equal(eventBinding(key({ key: "Shift" }), "windows"), null);
});

test("effective table: override replaces the whole group, [] stays cleared", () => {
  assert.deepEqual(effectiveBindings("refresh", {}, {}), ["Ctrl+R"]);
  assert.deepEqual(effectiveBindings("refresh", { refresh: ["F8"] }, {}), ["F8"]);
  assert.deepEqual(effectiveBindings("refresh", { refresh: [] }, {}), []);
  assert.deepEqual(effectiveBindings("wake", { wake: ["F9"] }, { shortcut: "Alt+9" }), ["Alt+9"]);
  assert.deepEqual(effectiveBindings("wake", { wake: ["F9"] }, {}), ["Alt+2"]);
});

test("legacy migration keeps explicit clears and ignores untouched commands", () => {
  assert.deepEqual(migrateLegacyShortcuts({ refresh: "F8", theme: "" }), {
    refresh: ["F8"],
    theme: [],
  });
  assert.deepEqual(migrateLegacyShortcuts(undefined), {});
});

test("conflicts are detected by physical equivalence and takeover preserves siblings", () => {
  const overrides = { refresh: ["Ctrl+R", "Ctrl+Shift+R"] };
  const conflict = findBindingConflict("settings", "cmdorctrl+r", overrides, {}, "windows");
  assert.equal(conflict?.id, "refresh");
  const next = takeoverBinding("settings", "Ctrl+R", overrides, "windows");
  assert.deepEqual(next.refresh, ["Ctrl+Shift+R"]);
  assert.equal(findBindingConflict("settings", "Ctrl+R", next, {}, "windows"), null);
  assert.equal(findBindingConflict("settings", "Ctrl+S", next, {}, "windows"), null);
});

test("reserved keys and bare-letter guard", () => {
  assert.equal(reservedBinding("Escape", "windows"), true);
  assert.equal(reservedBinding("F5", "windows"), true);
  assert.equal(reservedBinding("Ctrl+C", "windows"), true);
  assert.equal(reservedBinding("Ctrl+S", "windows"), false);
  assert.equal(reservedBinding("T", "windows"), false);
  const bare = recordBinding(key({ key: "t", code: "KeyT" }), "windows");
  assert.deepEqual(bare, { binding: "T", bare: true, reserved: false });
  const combo = recordBinding(key({ ctrlKey: true, key: "s", code: "KeyS" }), "windows");
  assert.deepEqual(combo, { binding: "CTRL+S", bare: false, reserved: false });
});

test("search matches labels and pressed combinations", () => {
  const commands = [
    { id: "refresh", labelKey: "Refresh", defaultBindings: ["Ctrl+R"], channel: "key", scope: "dashboard", configurable: true },
    { id: "theme", labelKey: "Theme", defaultBindings: ["Ctrl+L"], channel: "key", scope: "dashboard", configurable: true },
  ];
  const labels = (command) => command.labelKey;
  assert.deepEqual(filterShortcutCommands(commands, "the", labels, {}, {}, "windows").map((c) => c.id), ["theme"]);
  assert.deepEqual(filterShortcutCommands(commands, "Ctrl+R", labels, {}, {}, "windows").map((c) => c.id), ["refresh"]);
  assert.deepEqual(filterShortcutCommands(commands, "", labels, {}, {}, "windows").length, 2);
});
