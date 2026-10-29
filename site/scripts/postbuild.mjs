// The English pages share the root layout, so their exported HTML still
// carries lang="zh-CN". Set the language on the exported files instead of
// duplicating the root layout: crawlers see the right attribute and the
// client never has to patch it.
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT = fileURLToPath(new URL("../out/", import.meta.url));

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(path)));
    else if (entry.name.endsWith(".html")) files.push(path);
  }
  return files;
}

const englishRoot = join(OUT, "en");
let changed = 0;
for (const file of await walk(englishRoot)) {
  const html = await readFile(file, "utf8");
  if (!html.includes('<html lang="zh-CN"')) continue;
  await writeFile(file, html.replace('<html lang="zh-CN"', '<html lang="en"'));
  changed += 1;
}
console.log(`[postbuild] lang=en on ${changed} English page(s)`);
