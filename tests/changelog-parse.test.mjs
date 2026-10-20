// node --test tests/changelog-parse.test.mjs — changelog parser lock.
//
// CHANGELOG.md uses `## 新增/变更/修复` at the SAME heading level as the
// version headers, so a parser that treats every `## ` block as a version
// emits phantom "v新增" sections and drops the actual items. These tests run
// the production functions (extracted from main.ts, same AST + vm pattern as
// refresh-state.test.mjs) against both a fixture and the real changelog.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const ast = ts.createSourceFile("main.ts", source, ts.ScriptTarget.Latest, true);
const picked = ast.statements
  .filter(
    (node) =>
      (ts.isFunctionDeclaration(node) &&
        ["parseChangelog", "parseChangelogItems"].includes(node.name?.text)) ||
      (ts.isVariableStatement(node) &&
        node.declarationList.declarations.some((d) => d.name.getText(ast) === "CHANGELOG_GROUPS")),
  )
  .map((node) => node.getText(ast));
assert.equal(picked.length, 3, "parser sources moved — update the extraction");
const parserCode = ts.transpileModule(picked.join("\n"), {
  compilerOptions: { target: ts.ScriptTarget.ES2022 },
}).outputText;

function parse(changelogRaw) {
  const context = vm.createContext({ changelogRaw });
  vm.runInContext(parserCode, context);
  return { sections: context.parseChangelog(), items: context.parseChangelogItems };
}

const fixture = `# Changelog

## Unreleased

未发布主题

## 新增

- 未发布条目

## 0.8.6 — 2026-10-18

架构深化 / 额度单位

## 新增

- **独立用量单位**：说明
  续行补充

## 变更

- 变更条目

## 修复

- 修复条目

## 0.8.5 — 2026-10-16

主题 B

## 新增

- 条目 B
`;

test("group headings fold into the version above and never become versions", () => {
  const { sections } = parse(fixture);
  // vm realms have their own Array prototype; spread into host arrays.
  assert.deepEqual([...sections.map((s) => s.version)], ["0.8.6", "0.8.5"]);
  assert.deepEqual([...sections.map((s) => s.date)], ["2026-10-18", "2026-10-16"]);
});

test("unreleased and its groups are skipped entirely", () => {
  const { sections } = parse(fixture);
  assert.ok(!sections.some((s) => s.version === "Unreleased"));
  assert.ok(!sections.flatMap((s) => s.items).some((i) => i.text.includes("未发布条目")));
});

test("items carry their group's type and the summary is the theme line", () => {
  const [latest, older] = parse(fixture).sections;
  assert.equal(latest.summary, "架构深化 / 额度单位");
  assert.deepEqual([...latest.items.map((i) => i.type)], ["added", "changed", "fixed"]);
  assert.deepEqual([...older.items.map((i) => i.type)], ["added"]);
  assert.equal(older.summary, "主题 B");
});

test("continuation lines fold back into their bullet", () => {
  const [latest] = parse(fixture).sections;
  assert.equal(latest.items[0].text, "**独立用量单位**：说明 续行补充");
});

function atLeast(version, floor) {
  const a = version.split(".").map(Number);
  const b = floor.split(".").map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if ((a[i] ?? 0) !== (b[i] ?? 0)) return (a[i] ?? 0) > (b[i] ?? 0);
  }
  return true;
}

test("real CHANGELOG: no phantom versions and grouped releases carry typed items", () => {
  const changelog = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8");
  const { sections } = parse(changelog);
  assert.ok(sections.length > 10, `unexpected section count ${sections.length}`);
  for (const section of sections) {
    assert.match(section.version, /^\d+\.\d+\.\d+$/, `phantom version: ${section.version}`);
    // Releases from 0.4.69 on use 新增/变更/修复 groups; older entries are
    // flat bullet lists (or the 0.1.0 paragraph), so they may stay untyped.
    if (atLeast(section.version, "0.4.69")) {
      assert.ok(section.items.length > 0, `v${section.version} parsed no items`);
      assert.ok(
        section.items.every((i) => i.type !== "other"),
        `v${section.version} produced untyped items`,
      );
    }
  }
  const v086 = sections.find((s) => s.version === "0.8.6");
  assert.deepEqual(
    [...new Set(v086.items.map((i) => i.type))].sort(),
    ["added", "changed", "fixed"],
  );
});
