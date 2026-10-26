#!/usr/bin/env node
// Family gatekeeper, changed-lines edition (#65-01/03). Shared files may not
// gain new provider-specific branches: an added line that special-cases a
// family must delegate to `providers::<family>::` or carry an explicit
// `// provider-specific: <reason>` marker. This deliberately checks the diff
// instead of the whole tree, so the 400+ pre-existing cases need no baseline
// file and cannot hide a new one.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

export const TARGET_FILES = [
  "src-tauri/src/lib.rs",
  "src-tauri/src/spend.rs",
  "src-tauri/src/auth_center.rs",
  "src-tauri/src/tray_projection.rs",
  "src-tauri/src/provider_dispatch.rs",
  "src/main.ts",
];

export function catalogFamilies(catalogSource) {
  return new Set(
    [...catalogSource.matchAll(/familyId:\s*"([a-z0-9-]+)"/g)].map((match) => match[1]),
  );
}

function markerReason(line) {
  const match = line.match(/\/\/\s*provider-specific:\s*(.+)$/);
  return match && match[1].trim().length >= 8 ? match[1].trim() : null;
}

/** Pure scanner for tests: returns one message per offending added line. */
export function scanAddedLines(file, addedLines, families) {
  const problems = [];
  for (const { line, lineNumber } of addedLines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("//") || trimmed.startsWith("*")) continue;
    // Tests and fixtures legitimately name every family.
    if (/assert|json!|serde_json::json|fixture|#\[test\]/.test(trimmed)) continue;
    if (markerReason(line)) continue;
    for (const family of families) {
      const escaped = family.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      const delegate = new RegExp(`providers::${escaped}::`);
      const specialCase = new RegExp(
        `(?:==\\s*"${escaped}"|"${escaped}"\\s*=>|starts_with\\("${escaped}"\\)|matches!\\([^\\n]*"${escaped}")`,
      );
      if (specialCase.test(line) && !delegate.test(line)) {
        problems.push(
          `${file}:${lineNumber} unjustified family construct (${family}); ` +
            `delegate to providers::${family}:: or add '// provider-specific: <reason>'`,
        );
        break;
      }
    }
  }
  return problems;
}

export function parseAddedLines(diffText) {
  const result = [];
  let file = "";
  for (const line of diffText.split(/\r?\n/)) {
    if (line.startsWith("+++ b/")) {
      file = line.slice(6);
      continue;
    }
    const match = line.match(/^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (match) {
      result.push({ file, start: Number(match[1]), lines: [] });
      continue;
    }
    const current = result.at(-1);
    if (current && line.startsWith("+") && !line.startsWith("+++")) {
      current.lines.push({ line: line.slice(1), lineNumber: current.start + current.lines.length });
    }
  }
  return result;
}

function main() {
  const base = process.argv[2] ?? process.env.GITHUB_BASE_REF ?? "origin/main";
  if (!base || /^0+$/.test(base)) return;
  const catalog = readFileSync(new URL("../src-tauri/src/provider_catalog.rs", import.meta.url), "utf8");
  const families = catalogFamilies(catalog);
  let diff = "";
  try {
    diff = execFileSync(
      "git",
      ["diff", "--unified=0", base, "HEAD", "--", ...TARGET_FILES],
      { encoding: "utf8", maxBuffer: 8 * 1024 * 1024 },
    );
  } catch (error) {
    console.error(`family gatekeeper: could not diff ${base}: ${error.message}`);
    process.exit(2);
  }
  const problems = [];
  for (const chunk of parseAddedLines(diff)) {
    if (!TARGET_FILES.includes(chunk.file)) continue;
    problems.push(...scanAddedLines(chunk.file, chunk.lines, families));
  }
  if (problems.length) {
    for (const problem of problems) console.error(`[family-gatekeeper] ${problem}`);
    process.exit(1);
  }
  console.log(`family gatekeeper: ${families.size} families, no new unjustified branches`);
}

if (import.meta.url === `file://${process.argv[1]?.replace(/\\/g, "/")}`) main();
