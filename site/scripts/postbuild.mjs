// The Chinese pages share the root layout, so their exported HTML still says
// lang="en". Set the language on the files instead of duplicating the layout.
import { constants } from "node:fs";
import { copyFile, readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const OUT = fileURLToPath(new URL("../out/", import.meta.url));

async function walk(dir, extension) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(path, extension)));
    else if (entry.name.endsWith(extension)) files.push(path);
  }
  return files;
}

let changed = 0;
for (const file of await walk(join(OUT, "zh"), ".html")) {
  const html = await readFile(file, "utf8");
  if (!html.includes('<html lang="en"')) continue;
  await writeFile(file, html.replace('<html lang="en"', '<html lang="zh-CN"'));
  changed += 1;
}
console.log(`[postbuild] lang=zh-CN on ${changed} Chinese page(s)`);

// On Windows, Next can emit a nested directory for a route's prefetch
// payload even though its client requests the dotted filename.
let aliases = 0;
for (const file of await walk(OUT, ".txt")) {
  const parts = relative(OUT, file).split(sep);
  const segment = parts.findIndex((part) => part.startsWith("__next."));
  if (segment < 0 || segment === parts.length - 1) continue;
  const alias = join(OUT, ...parts.slice(0, segment), parts.slice(segment).join("."));
  try {
    await copyFile(file, alias, constants.COPYFILE_EXCL);
    aliases += 1;
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
  }
}
if (aliases) console.log(`[postbuild] added ${aliases} dotted route payloads`);
