import assert from "node:assert/strict";
import fs from "node:fs";

import ts from "typescript";

const source = fs.readFileSync("src/content/comparisons.ts", "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext },
}).outputText;
const { comparisons, isComparisonIndexable } = await import(
  `data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`
);
const sitemap = fs.readFileSync(".next/server/app/sitemap.xml.body", "utf8");
const plain = fs.readFileSync(".next/server/app/compare/llms.txt.body", "utf8");
const index = fs.readFileSync(".next/server/app/compare.html", "utf8");
assert.equal(new Set(comparisons.map((item) => item.slug)).size, comparisons.length);
assert.equal(isComparisonIndexable({}), false);
assert.equal(isComparisonIndexable({ research: { checkedOn: "today", methodology: "review", sources: [] } }), false);

const results = comparisons.map((comparison) => {
  const path = `/compare/${comparison.slug}`;
  const html = fs.readFileSync(`.next/server/app${path}.html`, "utf8");
  const indexable = isComparisonIndexable(comparison);
  assert.equal(html.includes('content="noindex, follow"'), !indexable, path);
  assert.equal(sitemap.includes(`https://designmode.app${path}</loc>`), indexable, path);
  assert.equal(plain.includes(`URL: https://designmode.app${path}\n`), indexable, path);
  assert(html.includes(`href="https://designmode.app${path}"`), path);
  assert(index.includes(`href="${path}"`), path);
  assert(html.includes("<table"), path);
  if (indexable) {
    assert(html.includes("Comparison sources"), path);
    for (const reference of comparison.research.sources) {
      assert(plain.includes(reference.url), path);
    }
  }
  for (const related of comparison.related) {
    assert(comparisons.some((item) => item.slug === related), `${path}: ${related}`);
  }
  return { path, indexable };
});
console.log(JSON.stringify({
  total: results.length,
  indexable: results.filter((item) => item.indexable).length,
  drafts: results.filter((item) => !item.indexable).length,
  results,
}, null, 2));
