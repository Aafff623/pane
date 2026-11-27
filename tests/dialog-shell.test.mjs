import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// `src/ui/dialogShell.ts` is the one place modal behaviour lives now. It exists
// because the 22 hand-written overlays each re-implemented Esc / Tab / focus
// return / scroll lock and got different subsets right: three trapped Tab, two
// returned focus, none locked the background.
//
// These tests run the real module against a fake DOM, so the behaviour asserted
// here is the behaviour the dialogs ship.

const src = readFileSync(new URL("../src/ui/dialogShell.ts", import.meta.url), "utf8");
// Transpile to plain ES2020 with no module wrapper, then hand the two exports
// back through the sandbox. The behaviour under test is ordinary JS once the
// type annotations are gone.
const js = ts
  .transpile(src, { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext })
  .replace(/^export\s+(function|const|type)/gm, "$1")
  .replace(/^export\s+\{[^}]*\};?$/gm, "");

/// Minimal DOM: enough element shape for the shell's queries and focus calls.
function makeEl(tag = "div") {
  const el = {
    tagName: tag.toUpperCase(),
    tag,
    id: "",
    className: "",
    style: {},
    children: [],
    parentNode: null,
    offsetParent: {},
    isConnected: true,
    listeners: new Map(),
    attributes: new Map(),
    setAttribute(k, v) { this.attributes.set(k, v); },
    getAttribute(k) { return this.attributes.get(k) ?? null; },
    addEventListener(type, fn) {
      const list = this.listeners.get(type) ?? [];
      list.push(fn);
      this.listeners.set(type, list);
    },
    removeEventListener(type, fn) {
      const list = this.listeners.get(type) ?? [];
      this.listeners.set(type, list.filter((f) => f !== fn));
    },
    dispatch(type, event) {
      for (const fn of this.listeners.get(type) ?? []) fn(event);
    },
    append(child) { this.children.push(child); child.parentNode = this; },
    appendChild(child) { this.append(child); },
    remove() {
      this.isConnected = false;
      if (this.parentNode) this.parentNode.children = this.parentNode.children.filter((c) => c !== this);
    },
    focus() { ctxActiveElement = this; },
    contains(node) {
      if (node === this) return true;
      return this.children.some((c) => c.contains?.(node));
    },
    querySelectorAll() { return this.focusList ?? []; },
    querySelector() { return (this.focusList ?? [])[0] ?? null; },
    focusList: [],
  };
  return el;
}

let ctxActiveElement = null;

const doc = {
  get activeElement() { return ctxActiveElement; },
  set activeElement(v) { ctxActiveElement = v; },
  body: null,
  listeners: new Map(),
  createElement: (tag) => makeEl(tag),
  addEventListener(type, fn) {
    const list = this.listeners.get(type) ?? [];
    list.push(fn);
    this.listeners.set(type, list);
  },
  removeEventListener(type, fn) {
    const list = this.listeners.get(type) ?? [];
    this.listeners.set(type, list.filter((f) => f !== fn));
  },
  dispatch(type, event) {
    for (const fn of this.listeners.get(type) ?? []) fn(event);
  },
};
doc.body = makeEl("body");

function load() {
  const ctx = vm.createContext({ document: doc, HTMLElement: class {}, console });
  vm.runInContext(js, ctx);
  return vm.runInContext("({ mountDialogShell, lockBackgroundScroll })", ctx);
}

function reset() {
  ctxActiveElement = null;
  doc.body.children = [];
  doc.listeners.clear();
}

test("the shell mounts into body and removes itself on close", () => {
  reset();
  const { mountDialogShell } = load();
  const shell = mountDialogShell();
  assert.ok(doc.body.children.includes(shell.host), "host must be mounted");
  shell.close();
  assert.equal(doc.body.children.includes(shell.host), false, "close unmounts it");
});

test("Escape closes the dialog and restores focus to the opener", () => {
  reset();
  const { mountDialogShell } = load();
  const trigger = makeEl("button");
  doc.activeElement = trigger;
  const shell = mountDialogShell();
  let closed = 0;
  const shellWithCallback = mountDialogShell({ onClose: () => { closed += 1; } });
  doc.dispatch("keydown", { key: "Escape", preventDefault() {}, stopPropagation() {} });
  assert.equal(shell.host.isConnected, false, "Escape removes the host");
  assert.equal(doc.activeElement, trigger, "focus returns to whatever opened it");
  assert.equal(closed, 1, "onClose fires exactly once");
  void shellWithCallback;
});

test("Tab wraps at both edges and never escapes a modal", () => {
  reset();
  const { mountDialogShell } = load();
  const shell = mountDialogShell();
  const first = makeEl("button");
  const middle = makeEl("input");
  const last = makeEl("button");
  // `focusables` reads the live DOM; stub the probe with the elements in order.
  shell.host.querySelectorAll = () => [first, middle, last];

  // Forward from the last node wraps to the first.
  doc.activeElement = last;
  let prevented = false;
  doc.dispatch("keydown", { key: "Tab", shiftKey: false, preventDefault() { prevented = true; } });
  assert.equal(prevented, true, "the browser's move is intercepted at the edge");
  assert.equal(doc.activeElement, first);

  // Backward from the first wraps to the last.
  doc.activeElement = first;
  doc.dispatch("keydown", { key: "Tab", shiftKey: true, preventDefault() {} });
  assert.equal(doc.activeElement, last);

  // In the middle the browser's own move is left alone.
  doc.activeElement = middle;
  prevented = false;
  doc.dispatch("keydown", { key: "Tab", shiftKey: false, preventDefault() { prevented = true; } });
  assert.equal(prevented, false, "middle-of-list Tab keeps native order");
});

test("a non-modal popover can opt out of the Tab trap", () => {
  reset();
  const { mountDialogShell } = load();
  const shell = mountDialogShell({ trapFocus: false });
  shell.host.querySelectorAll = () => [makeEl("button")];
  let prevented = false;
  doc.dispatch("keydown", { key: "Tab", shiftKey: false, preventDefault() { prevented = true; } });
  assert.equal(prevented, false);
});

test("closing twice is a no-op and detaches the key listener", () => {
  reset();
  const { mountDialogShell } = load();
  const shell = mountDialogShell();
  shell.close();
  const listeners = doc.listeners.get("keydown") ?? [];
  assert.equal(listeners.length, 0, "Escape handling must not outlive the dialog");
  shell.close();
  assert.equal(shell.host.isConnected, false);
});

test("a detached opener is not focused after the board re-rendered", () => {
  reset();
  const { mountDialogShell } = load();
  const trigger = makeEl("button");
  trigger.isConnected = false;
  doc.activeElement = trigger;
  const shell = mountDialogShell();
  const before = doc.activeElement;
  shell.close();
  assert.equal(doc.activeElement, before, "a replaced button must not steal focus");
});

test("background scrolling is locked while a modal is open", () => {
  reset();
  const { lockBackgroundScroll } = load();
  doc.body.style.overflow = "";
  const unlock = lockBackgroundScroll();
  assert.equal(doc.body.style.overflow, "hidden");
  unlock();
  assert.equal(doc.body.style.overflow, "", "the previous value comes back");
  unlock();
  assert.equal(doc.body.style.overflow, "", "unlocking twice is safe");
});

test("canClose vetoes a dismissal but not an explicit close", () => {
  reset();
  const { mountDialogShell } = load();
  let safe = false;
  const shell = mountDialogShell({ canClose: () => safe });
  doc.dispatch("keydown", { key: "Escape", preventDefault() {}, stopPropagation() {} });
  assert.equal(shell.host.isConnected, true, "Esc must not destroy an in-flight write");
  shell.host.dispatch("click", { target: shell.host });
  assert.equal(shell.host.isConnected, true, "nor may the backdrop");
  // The caller's own close is the "the write landed" path and always works.
  shell.close();
  assert.equal(shell.host.isConnected, false);
});
