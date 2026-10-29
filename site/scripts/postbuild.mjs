// The Chinese pages share the root layout, so their exported HTML still says
// lang="en". Set the language on the files instead of duplicating the layout.
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

let changed = 0;
for (const file of await walk(join(OUT, "zh"))) {
  const html = await readFile(file, "utf8");
  if (!html.includes('<html lang="en"')) continue;
  await writeFile(file, html.replace('<html lang="en"', '<html lang="zh-CN"'));
  changed += 1;
}
console.log(`[postbuild] lang=zh-CN on ${changed} Chinese page(s)`);
