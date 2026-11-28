import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// The A–Z rail is `position: fixed` and placed by `placeIndexRail()`.
//
// The bug this file pins: `getBoundingClientRect()` reports VISUAL pixels under
// `body.ui-scaled { zoom: var(--ui-scale) }`, while `offsetWidth`, `style.left`
// and `style.top` are LAYOUT pixels. Mixing the two pushed the rail off the
// column at any scale ≠ 1 — and it passed type checking, every unit test, and a
// look at the code, because both halves are individually correct.

const path = new URL("../src/main.ts", import.meta.url);
const source = ts.createSourceFile(path.pathname, readFileSync(path, "utf8"), ts.ScriptTarget.Latest, true);

function runFunction(name, globals) {
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

/// A rect as `scaledUiRect` receives it — visual pixels at the given scale.
function rect(left, top, width, height, scale) {
  return {
    left: left * scale,
    top: top * scale,
    right: (left + width) * scale,
    bottom: (top + height) * scale,
    width: width * scale,
    height: height * scale,
  };
}

function makeEl(rectValue, size) {
  return {
    style: {
      _v: {},
      set left(v) { this._v.left = v; },
      get left() { return this._v.left; },
      set top(v) { this._v.top = v; },
      get top() { return this._v.top; },
      getPropertyValue(k) { return this._v[k]; },
      setProperty(k, v) { this._v[k] = v; },
    },
    offsetWidth: size.width,
    offsetHeight: size.height,
    getBoundingClientRect: () => rectValue,
  };
}

function place(scale) {
  // Layout geometry: panel scroll area starts at x=20,y=40 and is 900×600
  // (CSS px). The card column is centred and 700 wide.
  const scroller = makeEl(rect(20, 40, 900, 600, scale), { width: 900, height: 600 });
  const column = makeEl(rect(120, 60, 700, 560, scale), { width: 700, height: 560 });
  const rail = makeEl(rect(0, 0, 29, 300, scale), { width: 29, height: 300 });
  const ctx = runFunction("placeIndexRail", {
    document: {
      querySelector(sel) {
        if (sel === "#skey-az") return rail;
        if (sel === "#panel-scroll") return scroller;
        if (sel === "#st-keyvault") return column;
        return null;
      },
    },
    currentUiScale: () => scale,
    scaledUiRect: (r) => ({
      left: r.left / scale,
      top: r.top / scale,
      right: r.right / scale,
      bottom: r.bottom / scale,
      width: r.width / scale,
      height: r.height / scale,
    }),
    Math,
    // The rail's own height drives the vertical clamp.
    requestAnimationFrame: (fn) => fn(),
  });
  ctx.placeIndexRail();
  return { left: rail.style.left, top: rail.style.top, gutter: column.style.getPropertyValue("--rail-gutter") };
}

test("the A–Z rail lands at the same layout position at every UI scale", () => {
  const at100 = place(1);
  const at125 = place(1.25);
  const at175 = place(1.75);

  assert.equal(at100.left, at125.left, "scale must not move the rail horizontally");
  assert.equal(at100.left, at175.left);
  assert.equal(at100.top, at125.top, "scale must not move the rail vertically");
  assert.equal(at100.top, at175.top);
});

test("the rail sits just right of the card column, in layout pixels", () => {
  const { left, gutter } = place(1.25);
  // Column right edge = 120 + 700 = 820; the rail goes 12px beyond it.
  assert.equal(left, "832px", "beside the column, not divided by scale again");
  assert.equal(gutter, "0px", "a 900-wide scroller leaves margin, so no gutter");
});

test("a narrow window reserves a gutter instead of covering the cards", () => {
  const scale = 1.25;
  // Scroller only 760 wide: 120 + 700 + 12 + 29 exceeds it, so the rail has to
  // sit inside the right edge and the column reserves room for it.
  const scroller = makeEl(rect(20, 40, 760, 600, scale), { width: 760, height: 600 });
  const column = makeEl(rect(50, 60, 700, 560, scale), { width: 700, height: 560 });
  const rail = makeEl(rect(0, 0, 29, 300, scale), { width: 29, height: 300 });
  const ctx = runFunction("placeIndexRail", {
    document: {
      querySelector(sel) {
        if (sel === "#skey-az") return rail;
        if (sel === "#panel-scroll") return scroller;
        if (sel === "#st-keyvault") return column;
        return null;
      },
    },
    currentUiScale: () => scale,
    scaledUiRect: (r) => ({
      left: r.left / scale,
      top: r.top / scale,
      right: r.right / scale,
      bottom: r.bottom / scale,
      width: r.width / scale,
      height: r.height / scale,
    }),
    Math,
    requestAnimationFrame: (fn) => fn(),
  });
  ctx.placeIndexRail();
  assert.equal(column.style.getPropertyValue("--rail-gutter"), "49px", "rail width 29 + 20 breathing room");
  // scroller right = 20 + 760 = 780; rail width 29, 12px inset → 739.
  assert.equal(rail.style.left, "739px", "just inside the scroller's right edge");
});
