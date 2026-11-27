import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// `appConfirm` / `appPrompt` are awaited by every destructive flow in the app
// (delete provider, delete account, archive, group delete). The shared dialog
// shell owns Escape and the backdrop, and its close path never runs the
// dialog's own `done()` — so the cancel value has to be settled from the
// shell's `onClose`. Before that, pressing Esc left these Promises pending
// forever and the delete flow simply stopped.
//
// These run the real functions against a fake DOM, so the behaviour asserted
// here is the behaviour the app ships.

const path = new URL("../src/main.ts", import.meta.url);
const source = ts.createSourceFile(path.pathname, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);

function runFunction(name, globals) {
  // Overloaded declarations (`appConfirm` has two) put the bodyless signatures
  // first, so pick the declaration that actually has a body.
  const fn = source.statements.find(
    (node) => ts.isFunctionDeclaration(node) && node.name?.text === name && node.body,
  );
  assert.ok(fn, `Expected ${name} in the app entry point`);
  const code = ts.transpile(fn.getText(source), {
    target: ts.ScriptTarget.ES2020,
    module: ts.ModuleKind.CommonJS,
  });
  const context = vm.createContext(globals);
  vm.runInContext(code, context);
  return context;
}

/// Minimal element: enough for `overlay.innerHTML = …` plus the `querySelector`
/// lookups the two dialogs make. Returning a shared stub for every selector is
/// fine — the tests drive `onClose`, not the buttons.
function makeOverlay() {
  const button = {
    addEventListener() {},
    focus() {},
    disabled: false,
    classList: { add() {}, remove() {}, toggle() {} },
    // `renderBody` writes into whatever `#whatsnew-body` returns and then
    // probes inside it, so the shared stub has to be queryable itself.
    querySelector: () => null,
    querySelectorAll: () => [],
    setAttribute() {},
  };
  const checkbox = { checked: false };
  const input = {
    value: "",
    placeholder: "",
    addEventListener() {},
    focus() {},
    select() {},
  };
  return {
    innerHTML: "",
    id: "",
    dataset: {},
    listeners: new Map(),
    querySelector(selector) {
      if (selector === "#confirm-checkbox") return checkbox;
      if (selector === "#prompt-input") return input;
      // `renderBody` probes optional nodes (the "earlier versions" expander);
      // an absent one must read as null, not as a button.
      if (selector === "#whatsnew-earlier") return null;
      return button;
    },
    querySelectorAll: () => [],
    addEventListener(type, fn) { this.listeners.set(type, fn); },
    setAttribute() {},
    remove() {},
    isConnected: true,
  };
}

/// The shell's contract, reduced to what these dialogs use: mounting records
/// the options, and `close()` invokes `onClose` (the real module does the same,
/// then restores focus and removes the node). `dismiss()` is what Escape and a
/// backdrop click reach — and crucially it does NOT go through the dialog's
/// own buttons, which is the path this file exists to cover.
function makeShellRecording() {
  return (options = {}) => {
    const host = makeOverlay();
    let closed = false;
    const shell = {
      host,
      opener: null,
      close: () => {
        if (closed) return;
        closed = true;
        options.onClose?.();
      },
      focusFirst() {},
    };
    lastShell = shell;
    return shell;
  };
}

/// Set by the factory above so a test can press Escape exactly the way the
/// real shell does.
let lastShell = null;

function pressEscape() {
  assert.ok(lastShell, "no dialog mounted");
  lastShell.close();
}

function baseGlobals() {
  return {
    document: { body: { append() {} }, querySelector: () => null },
    lockBackgroundScroll: () => () => {},
    mountDialogShell: makeShellRecording(),
    escapeHtml: (text) => text,
    t: (key) => key,
    dismissConfirm: null,
  };
}

test("appConfirm settles its Promise as cancelled when Escape closes it", async () => {
  const ctx = runFunction("appConfirm", baseGlobals());
  const pending = ctx.appConfirm({ title: "Delete", message: "Sure?", confirmLabel: "Delete" });
  pressEscape();
  assert.equal(await pending, false, "cancelled confirm must resolve false, not hang");
});

test("appConfirm with a checkbox resolves an object, not a bare boolean", async () => {
  const ctx = runFunction("appConfirm", baseGlobals());
  const pending = ctx.appConfirm({
    title: "Delete",
    message: "Sure?",
    confirmLabel: "Delete",
    checkboxLabel: "Keep usage",
    checkboxDefault: true,
  });
  pressEscape();
  const result = await pending;
  assert.equal(result.ok, false);
  assert.equal(result.checked, false, "a cancelled dialog reports nothing checked");
});

test("appPrompt resolves null when Escape closes it", async () => {
  const ctx = runFunction("appPrompt", baseGlobals());
  const pending = ctx.appPrompt({ title: "Rename", confirmLabel: "OK" });
  pressEscape();
  assert.equal(await pending, null, "cancelled prompt must resolve null, not hang");
});

test("a late second close does not re-settle a confirmed dialog", async () => {
  const ctx = runFunction("appConfirm", baseGlobals());
  const pending = ctx.appConfirm({ title: "Delete", message: "Sure?", confirmLabel: "Delete" });
  // OK button first, then a stray close from a popover re-render.
  ctx.dismissConfirm?.();
  assert.equal(await pending, false);
  lastShell.close();
  assert.equal(await pending, false, "the first settle wins");
});

test("the changelog's shell close runs the notice callback exactly once", async () => {
  // Skipping this callback left `noticeOpen` true for the rest of the
  // session, which blocked every later boot notice and the first-run tour.
  let resumed = 0;
  const overlay = makeOverlay();
  const shellMounts = [];
  const ctx = runFunction("showChangelogDialog", {
    document: {
      body: { append() {} },
      querySelector: () => null,
      hidden: false,
      querySelectorAll: () => [],
      createElement: () => makeOverlay(),
    },
    mountDialogShell: (options) => {
      shellMounts.push(options);
      return { host: overlay, opener: null, close: () => options.onClose?.(), focusFirst() {} };
    },
    lockBackgroundScroll: () => () => {},
    escapeHtml: (text) => text,
    t: (key) => key,
    uiIcon: () => "",
    dismissWhatsNew: null,
    renderChangelogBody: () => "",
    renderChangelogSection: () => "",
    parseChangelog: () => [],
    WHATS_NEW_VERSIONS: 3,
    pendingFontGuide: null,
    maybeStartTour: () => {},
  });
  ctx.showChangelogDialog("Changelog", [], { onClose: () => { resumed += 1; } });
  assert.equal(shellMounts.length, 1);
  // What the shell does on Escape / backdrop.
  shellMounts[0].onClose?.();
  assert.equal(resumed, 1, "Esc must resume the notice queue");
  // A second close is a no-op rather than a second resume.
  shellMounts[0].onClose?.();
  assert.equal(resumed, 1);
});
