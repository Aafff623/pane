// node --test tests/spend-fold-aliases.test.mjs — alias folding contract.
//
// SPEND_FOLD aliases rows onto one product before the donut sees them.
// Sum aliases (claude-desktop→claude, minimax→mcode) are different ledgers
// and must add up regardless of which row fetch_spend lists first; the old
// order-dependent path could replace a summed row with the larger-token
// alias and silently drop the dollars.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const ast = ts.createSourceFile("main.ts", source, ts.ScriptTarget.Latest, true);
const wanted = ["SPEND_FOLD", "SPEND_SUM_GROUPS"];
const nodes = ast.statements.filter(
  (node) =>
    (ts.isVariableStatement(node) &&
      node.declarationList.declarations.some((d) => wanted.includes(d.name.getText(ast)))) ||
    (ts.isFunctionDeclaration(node) && ["foldSpendId", "foldRows"].includes(node.name?.text)),
);
assert.equal(nodes.length, 4);

const context = vm.createContext({});
// ES2020 keeps native spread — the default ES5 emit turns `[...iter]` into a
// helper that cannot iterate a Map iterator and yields [].
vm.runInContext(
  ts.transpile(nodes.map((node) => node.getText(ast)).join("\n"), {
    target: ts.ScriptTarget.ES2020,
  }),
  context,
);

const row = (id, cost, tokens) => ({ id, cost, tokens });
const merge = (a, b) => row(a.id, a.cost + b.cost, a.tokens + b.tokens);
const fold = (rows) =>
  context.foldRows(
    rows,
    (r) => r.tokens,
    merge,
    (r, id) => ({ ...r, id }),
  );

test("sum aliases add dollars whichever row comes first", () => {
  const minimax = row("minimax", 40.27, 685_011_648);
  const mcode = row("mcode", 0, 912_696_312);
  for (const rows of [
    [minimax, mcode],
    [mcode, minimax],
  ]) {
    const out = fold(rows);
    assert.equal(out.length, 1);
    assert.equal(out[0].id, "mcode");
    assert.equal(out[0].cost, 40.27);
    assert.equal(out[0].tokens, 1_597_707_960);
  }
});

test("same-ledger aliases keep the larger row, not their sum", () => {
  const kimi = row("kimi", 5, 900_000);
  const moonshot = row("moonshot", 5, 1_200_000);
  for (const rows of [
    [kimi, moonshot],
    [moonshot, kimi],
  ]) {
    const out = fold(rows);
    assert.equal(out.length, 1);
    assert.equal(out[0].id, "moonshot");
    assert.equal(out[0].tokens, 1_200_000);
  }
});
