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

// #43-2 — pricing_mode was computed in Rust and typed in TypeScript but never
// rendered, so a reader could not tell a vendor-displayed figure from one Pane
// computed itself. The badge must cover every mode the server can send, and it
// must stay silent for the normal case or it would chip every card.
test("the pricing badge names where the dollars came from (#43-2)", () => {
  assert.ok(source.includes('const pricing = sp.pricing_mode ?? "auto";'), "the mode is read");
  assert.ok(source.includes('if (pricing !== "auto")'), "a catalog match stays unbadged");
  assert.ok(source.includes("`spend.pricing.${pricing}`"), "the label is keyed by the server's mode");

  const i18n = readFileSync(new URL("../src/i18n.ts", import.meta.url), "utf8");
  const rust = readFileSync(new URL("../src-tauri/src/spend.rs", import.meta.url), "utf8");
  // The enum is `rename_all = "snake_case"`, so these four strings are the whole
  // wire vocabulary. A mode without a label would render a raw key on the card.
  assert.match(rust, /#\[serde\(rename_all = "snake_case"\)\]\s*pub enum PricingMode/);
  const enumBlock = rust.slice(rust.indexOf("pub enum PricingMode"), rust.indexOf("}", rust.indexOf("pub enum PricingMode")));
  for (const mode of ["display", "auto", "calculate", "unknown"]) {
    assert.ok(i18n.includes(`"spend.pricing.${mode}"`), `missing label for ${mode}`);
    const variant = mode[0].toUpperCase() + mode.slice(1);
    assert.ok(enumBlock.includes(variant), `${variant} is a declared PricingMode`);
  }
});
