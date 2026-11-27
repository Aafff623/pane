import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// `pillControl` draws the segmented "pick one of N" rows in Settings (text
// size, colour-blind mode, meter direction, window form). The hand-written
// version only responded to the mouse: a keyboard user could tab into the row
// and then had no way to change the answer, and the control announced itself as
// a generic group rather than as mutually exclusive options.
//
// The fix is in the shared factory, so every row inherits it. These tests run
// the real function against a stub DOM.

const src = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const main = ts.createSourceFile("main.ts", src, ts.ScriptTarget.ES2020, true);
const wanted = new Set(["pillControl"]);
const js = ts
  .transpile(
    main.statements
      .filter((node) => ts.isFunctionDeclaration(node) && wanted.has(node.name?.text))
      .map((node) => node.getText(main))
      .join("\n"),
    { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext },
  )
  .replace(/^export\s+(function|const|type)/gm, "$1");

function makeEl(tag = "div") {
  const el = {
    tag,
    className: "",
    textContent: "",
    dataset: {},
    tabIndex: 0,
    style: {},
    children: [],
    attributes: new Map(),
    listeners: new Map(),
    setAttribute(k, v) { this.attributes.set(k, v); },
    getAttribute(k) { return this.attributes.get(k) ?? null; },
    hasAttribute(k) { return this.attributes.has(k); },
    addEventListener(type, fn) {
      const list = this.listeners.get(type) ?? [];
      list.push(fn);
      this.listeners.set(type, list);
    },
    dispatch(type, event) {
      for (const fn of this.listeners.get(type) ?? []) fn(event);
    },
    append(child) { this.children.push(child); child.parent = this; },
    classList: {
      toggle(_c, on) { void on; },
      contains() { return false; },
    },
    focus() { ctx.activeElement = this; },
    getBoundingClientRect: () => ({ width: 60, left: 0, top: 0, height: 20, right: 60, bottom: 20 }),
  };
  // classList.has to be per-element so `sync` can drive it.
  const active = { on: false };
  el.classList = {
    toggle(_c, state) { active.on = state; },
    contains() { return active.on; },
  };
  return el;
}

const ctx = {
  document: { createElement: (tag) => makeEl(tag), activeElement: null },
  requestAnimationFrame: (fn) => fn(),
  ResizeObserver: class { observe() {} },
  positionTabThumb: () => {},
};
ctx.document.createElement = (tag) => makeEl(tag);

function load() {
  const sandbox = vm.createContext(ctx);
  vm.runInContext(js, sandbox);
  return vm.runInContext("pillControl", sandbox);
}

function build() {
  const calls = [];
  const pillControl = load();
  const pill = pillControl(
    [{ value: "a", label: "A" }, { value: "b", label: "B" }],
    "a",
    (v) => calls.push(v),
    "Choose one",
  );
  return { pill, calls };
}

test("the segmented control announces itself as a radiogroup", () => {
  const { pill } = build();
  assert.equal(pill.getAttribute("role"), "radiogroup");
  assert.equal(pill.getAttribute("aria-label"), "Choose one");
  const radios = pill.children.filter((c) => c.getAttribute("role") === "radio");
  assert.equal(radios.length, 2, "every option is a radio");
});

test("only the current option is a tab stop", () => {
  const { pill } = build();
  const radios = pill.children.filter((c) => c.getAttribute("role") === "radio");
  assert.equal(radios[0].tabIndex, 0, "the selected option is focusable");
  assert.equal(radios[1].tabIndex, -1, "the rest are reachable by arrow keys only");
  assert.equal(radios[0].getAttribute("aria-checked"), "true");
  assert.equal(radios[1].getAttribute("aria-checked"), "false");
});

test("arrow keys move the selection and fire onChange", () => {
  const { pill, calls } = build();
  const radios = pill.children.filter((c) => c.getAttribute("role") === "radio");
  ctx.document.activeElement = radios[0];
  let prevented = false;
  pill.dispatch("keydown", { key: "ArrowRight", preventDefault() { prevented = true; } });
  assert.equal(prevented, true, "the browser's own scroll is suppressed");
  assert.deepEqual(calls, ["b"], "the next option is committed");
});

test("Home and End jump to the ends, and the ring wraps", () => {
  const { pill, calls } = build();
  const radios = pill.children.filter((c) => c.getAttribute("role") === "radio");
  ctx.document.activeElement = radios[1];
  pill.dispatch("keydown", { key: "ArrowRight", preventDefault() {} });
  assert.deepEqual(calls, ["a"], "right from the last wraps to the first");
  ctx.document.activeElement = radios[0];
  pill.dispatch("keydown", { key: "ArrowLeft", preventDefault() {} });
  assert.deepEqual(calls, ["a", "b"], "left from the first wraps to the last");
  ctx.document.activeElement = radios[0];
  pill.dispatch("keydown", { key: "End", preventDefault() {} });
  assert.equal(calls.at(-1), "b");
  pill.dispatch("keydown", { key: "Home", preventDefault() {} });
  assert.equal(calls.at(-1), "a");
});

test("a key that is not a navigation key is left alone", () => {
  const { pill, calls } = build();
  ctx.document.activeElement = pill.children.filter((c) => c.getAttribute("role") === "radio")[0];
  let prevented = false;
  pill.dispatch("keydown", { key: "Tab", preventDefault() { prevented = true; } });
  assert.equal(prevented, false, "Tab must still leave the control");
  assert.deepEqual(calls, [], "and must not commit anything");
});
