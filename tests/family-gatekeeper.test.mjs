import test from "node:test";
import assert from "node:assert/strict";
import {
  catalogFamilies,
  parseAddedLines,
  scanAddedLines,
} from "../scripts/family-gatekeeper.mjs";

const families = new Set(["claude", "codex"]);

test("a new family special case must delegate or carry a reason", () => {
  const problems = scanAddedLines(
    "src-tauri/src/lib.rs",
    [
      { line: 'if family == "claude" { count += 1; }', lineNumber: 10 },
      { line: 'let x = providers::codex::fetch();', lineNumber: 11 },
      { line: 'if family == "codex" { // provider-specific: port fallback', lineNumber: 12 },
      { line: 'assert_eq!(fam, "claude");', lineNumber: 13 },
    ],
    families,
  );
  assert.equal(problems.length, 1);
  assert.match(problems[0], /lib\.rs:10.*claude/);
});

test("diff parsing tracks target files and added lines only", () => {
  const diff = [
    "+++ b/src-tauri/src/lib.rs",
    "@@ -3,0 +4,2 @@",
    '+if family == "claude" {',
    "+providers::codex::fetch();",
    "+++ b/src/main.ts",
    "@@ -10 +10 @@",
    '+const x = family === "codex";',
  ].join("\n");
  const parsed = parseAddedLines(diff);
  assert.equal(parsed.length, 2);
  assert.equal(parsed[0].file, "src-tauri/src/lib.rs");
  assert.equal(parsed[0].lines[0].lineNumber, 4);
  assert.equal(parsed[1].lines[0].line, 'const x = family === "codex";');
});

test("the catalog is the single family source", () => {
  const list = catalogFamilies('family_id: "x", familyId: "claude", familyId: "zcode"');
  assert.deepEqual([...list], ["claude", "zcode"]);
});
