#!/usr/bin/env node
// Release gate (#62-04): a tag must match package.json and have a non-empty
// CHANGELOG section. The release body is still authored from CHANGELOG by
// docs/release-format.md; this only makes a missing/mismatched section fail
// before three platform installers are built.
import { readFileSync } from "node:fs";

const tag = (process.argv[2] ?? "").trim().replace(/^v/, "");
if (!tag) {
  console.error("release-notes gate: pass the tag version (v0.10.0)");
  process.exit(2);
}
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
if (pkg.version !== tag) {
  console.error(`release-notes gate: package.json ${pkg.version} ≠ tag ${tag}`);
  process.exit(1);
}
const changelog = readFileSync(new URL("../CHANGELOG.md", import.meta.url), "utf8");
const lines = changelog.split(/\r?\n/);
const start = lines.findIndex((line) => line.startsWith(`## ${tag} `) || line === `## ${tag}`);
if (start < 0) {
  console.error(`release-notes gate: CHANGELOG has no "## ${tag}" section`);
  process.exit(1);
}
const body = [];
for (let i = start + 1; i < lines.length; i += 1) {
  // `## 新增` / `## 变更` are subsections; only the next version heading ends
  // this release's content.
  if (/^##\s+v?\d+\.\d+\.\d+/.test(lines[i])) break;
  body.push(lines[i]);
}
if (body.join("\n").replace(/^#+\s*/gm, "").trim().length < 40) {
  console.error(`release-notes gate: "## ${tag}" section is empty or too short`);
  process.exit(1);
}
console.log(`release-notes gate: ${tag} present in CHANGELOG`);
