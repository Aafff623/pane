import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const website = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const read = (file) => readFile(path.join(website, file), "utf8");
const [page, guide, skill, index, local, cloud] = await Promise.all([
  read("src/app/mcp/page.tsx"),
  read("src/lib/mcp-guide.ts"),
  read("public/.well-known/agent-skills/design-mode/SKILL.md"),
  read("public/.well-known/agent-skills/index.json"),
  read("../packages/mcp-local/src/mcp-server.ts"),
  read("../packages/mcp-cloud/api/mcp.ts"),
]);
const localTools = [...local.matchAll(/server\.tool\(\s*'([^']+)'/g)]
  .map((m) => m[1])
  .sort();
const cloudTools = [...cloud.matchAll(/name: '([^']+)',\s*description:/g)]
  .map((m) => m[1])
  .sort();
const pageTools = [...page.matchAll(/name: "([^"]+)"/g)]
  .map((m) => m[1])
  .sort();
assert.deepEqual(pageTools, cloudTools);
assert.deepEqual(localTools, [...cloudTools, "wait_for_handoff"].sort());
for (const name of localTools) assert(skill.includes("`" + name + "`"), name);
assert(skill.startsWith("---\nname: design-mode\n"));
assert.match(skill, /\ndescription: Use when .{1,60}\n/);
assert.match(skill, /\n---\n\n# Design Mode/);
assert.equal(
  JSON.parse(index).skills[0].url,
  "https://designmode.app/.well-known/agent-skills/design-mode/SKILL.md",
);
assert(guide.includes("https://mcp.designmode.app/mcp"));
assert(
  guide.includes("${DESIGN_MODE_TOKEN}") &&
    guide.includes("${env:DESIGN_MODE_TOKEN}") &&
    guide.includes("${input:designModeToken}"),
);
assert(!/npx\s+(-y\s+)?@designmode-app\/cli/.test(page + guide + skill));
for (const file of ["public/llms.txt", "public/llms-full.txt"]) {
  const text = await read(file);
  assert(
    text.includes(
      "https://designmode.app/.well-known/agent-skills/design-mode/SKILL.md",
    ),
  );
  assert(
    text.includes("https://designmode.app/.well-known/agent-skills/index.json"),
  );
}
const homepage = await read("src/app/page.tsx");
assert(homepage.indexOf("<FAQ />") < homepage.indexOf("<BrowserHandoff />"));
assert(
  homepage.indexOf("<BrowserHandoff />") < homepage.indexOf("<ClosingCta />"),
);
const image = await readFile(path.join(website, "public/og-design-mode-inter-v3.png"));
assert.equal(image.readUInt32BE(16), 1200);
assert.equal(image.readUInt32BE(20), 630);
assert.deepEqual(image, await readFile(path.join(website, "public/og-image.png")));
console.log(
  `MCP website contract passed: ${cloudTools.length} shared tools, ${localTools.length} Local tools; skill discovery and homepage order verified.`,
);
