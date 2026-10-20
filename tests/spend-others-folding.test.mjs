// node --test tests/spend-others-folding.test.mjs — Others folding contract.
//
// Cost mode folds sub-threshold providers into one "Others" wedge; the
// spendShowAll setting lifts that fold entirely, and token views never fold.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const ast = ts.createSourceFile("main.ts", source, ts.ScriptTarget.Latest, true);
const names = ["foldDonutEntries", "othersFoldUsd", "spendVal"];
const functions = ast.statements.filter(
  (node) => ts.isFunctionDeclaration(node) && names.includes(node.name?.text),
);
assert.equal(functions.length, names.length);

const config = { spendMetric: "cost", spendShowAll: false };
const context = vm.createContext({
  OTHERS_ID: "__others__",
  config,
  t: (key) => key,
});
vm.runInContext(ts.transpile(functions.map((node) => node.getText(ast)).join("\n")), context);

const entry = (id, cost, tokens = 1_000_000) => ({
  s: { id, name: id },
  w: { cost, tokens, models: [{ model: `${id}-model`, cost, tokens }] },
});

test("cost mode folds sub-threshold providers into one Others wedge", () => {
  const out = context.foldDonutEntries(
    [entry("big", 100), entry("small-a", 2), entry("small-b", 1)],
    "today",
  );
  assert.equal(out.length, 2);
  assert.equal(out[1].s.id, "__others__");
  assert.deepEqual(
    out[1].parts.map((p) => p.id),
    ["small-a", "small-b"],
  );
});

test("spendShowAll keeps every provider its own wedge in cost mode", () => {
  config.spendShowAll = true;
  const out = context.foldDonutEntries([entry("big", 100), entry("small-a", 2)], "today");
  config.spendShowAll = false;
  assert.equal(out.length, 2);
  assert.ok(!out.some((e) => e.s.id === "__others__"));
});

test("token mode never folds, with or without spendShowAll", () => {
  config.spendMetric = "tokens";
  const out = context.foldDonutEntries([entry("big", 100), entry("small-a", 2)], "today");
  config.spendMetric = "cost";
  assert.equal(out.length, 2);
  assert.ok(!out.some((e) => e.s.id === "__others__"));
});
