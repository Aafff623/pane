// node --test tests/spend-estimated.test.mjs — estimated-token label contract.
//
// The headline token number must be able to say how much of itself is an
// estimate; this helper is the single seam that decides that amount.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const ast = ts.createSourceFile("main.ts", source, ts.ScriptTarget.Latest, true);
const fn = ast.statements.find(
  (node) => ts.isFunctionDeclaration(node) && node.name?.text === "estimatedTokensFor",
);
assert.ok(fn, "estimatedTokensFor must exist in src/main.ts");

const context = vm.createContext({});
vm.runInContext(ts.transpile(fn.getText(ast)), context);
const estimatedTokensFor = context.estimatedTokensFor;

test("measured rows contribute nothing to the estimate", () => {
  assert.equal(
    estimatedTokensFor([
      { estimated: false, w: { tokens: 100 } },
      { w: { tokens: 50 } },
    ]),
    0,
  );
});

test("mixed rows report only the derived tokens", () => {
  assert.equal(
    estimatedTokensFor([
      { estimated: true, w: { tokens: 120 } },
      { estimated: false, w: { tokens: 50 } },
      { estimated: true, w: { tokens: 30 } },
    ]),
    150,
  );
});

test("an all-estimated or empty view stays coherent", () => {
  assert.equal(estimatedTokensFor([{ estimated: true, w: { tokens: 120 } }]), 120);
  assert.equal(estimatedTokensFor([]), 0);
});

test("by-model slices report only their derived share", () => {
  assert.equal(
    estimatedTokensFor([{ estimated: true, estTokens: 50, w: { tokens: 200 } }]),
    50,
    "measured tokens sharing the model name must not be counted as estimated",
  );
});
