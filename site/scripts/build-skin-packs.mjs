// Builds the official skin packs into site/public/skins/ (+ index.json).
//
//   node site/scripts/build-skin-packs.mjs [version]
//
// Each pack = the skin's stylesheet (dual-scoped so workshop previews theme
// exactly like the applied skin) + its assets, zipped with a manifest whose
// files list carries a sha256 per member — the format skin_packs.rs
// verifies. Zips are built locally (pwsh Compress-Archive, forward-slash
// entries) and COMMITTED beside the site; CI only deploys them, so there is
// no extra infrastructure beyond the existing site pipeline.
import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir, rm, readdir, copyFile, stat } from "node:fs/promises";
import { join, resolve, dirname, relative } from "node:path";
import { fileURLToPath } from "node:url";

const version = process.argv[2] || "1.0.0";
const site = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repo = resolve(site, "..");
const origin = "https://pane.threetwoa.live";
const outDir = join(site, "public", "skins");

// Pack sources live outside the app bundle: `skins/<id>/skin.css` plus its
// own `assets/` tree. Signal uses app-relative asset tokens that this script
// rewrites; ink's stylesheet already carries the dual root/sample scope.
const SKINS = [
  { id: "signal", name: "Cloud Post Office", css: "skins/signal/skin.css", assets: "skins/signal/assets", assetPrefix: "cloud-post" },
  { id: "ink", name: "山水笺 Ink Wash", css: "skins/ink/skin.css", assets: "skins/ink/assets", assetPrefix: "ink-wash" },
];

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

// Signal's stylesheet addresses the scoped root only; the pack (and the
// workshop preview) renders inside a .skin-sample[data-skin] container, so
// every scope token becomes self-or-container — the same dual scope
// inkSkin.css already ships with.
function dualScope(css, id) {
  return css
    .split(`:root[data-skin="${id}"]`).join(`:is(:root[data-skin="${id}"],[data-skin="${id}"].skin-sample)`)
    .split(`[data-skin="${id}"]`).join(`:is([data-skin="${id}"],[data-skin="${id}"].skin-sample)`);
}

// App-relative asset URLs become pack-relative ("assets/…").
function rewriteUrls(css, assetPrefix) {
  return css.split(`./assets/skins/${assetPrefix}/`).join("assets/").split(`assets/skins/${assetPrefix}/`).join("assets/");
}

async function collect(dir, base = dir) {
  const files = [];
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) files.push(...await collect(p, base));
    else files.push(relative(base, p).split("\\").join("/"));
  }
  return files;
}

await rm(outDir, { recursive: true, force: true });
await mkdir(outDir, { recursive: true });

const index = [];
for (const skin of SKINS) {
  const packDir = join(outDir, `_pack-${skin.id}`);
  await rm(packDir, { recursive: true, force: true });
  await mkdir(join(packDir, "assets"), { recursive: true });

  let css = await readFile(resolve(repo, skin.css), "utf8");
  if (skin.id === "signal") css = dualScope(css, skin.id);
  css = rewriteUrls(css, skin.assetPrefix);
  await writeFile(join(packDir, "skin.css"), css);

  for (const rel of await collect(resolve(repo, skin.assets))) {
    const target = join(packDir, "assets", rel);
    await mkdir(dirname(target), { recursive: true });
    await copyFile(resolve(repo, skin.assets, rel), target);
  }

  const files = [{ path: "skin.css", sha256: sha256(await readFile(join(packDir, "skin.css"))) }];
  for (const rel of await collect(join(packDir, "assets"))) {
    files.push({
      path: `assets/${rel}`,
      sha256: sha256(await readFile(join(packDir, "assets", rel))),
    });
  }
  const manifest = {
    id: skin.id,
    name: skin.name,
    version,
    min_app_version: "0.7.0",
    entry: "skin.css",
    files,
  };
  await writeFile(join(packDir, "manifest.json"), JSON.stringify(manifest, null, 2));

  const zipName = `pane-skin-${skin.id}-${version}.zip`;
  const zipPath = join(outDir, zipName);
  await rm(zipPath, { force: true });
  // -Recurse without a wildcard would flatten; the trailing \* keeps the
  // folder structure, and pwsh 7 writes forward-slash entry names.
  const ps = `Compress-Archive -Path '${packDir}\\*' -DestinationPath '${zipPath}' -CompressionLevel Optimal -Force`;
  const { execSync } = await import("node:child_process");
  execSync(`pwsh -NoProfile -Command "${ps.replace(/"/g, '\\"')}"`, { stdio: "inherit" });

  const bytes = await readFile(zipPath);
  index.push({
    id: skin.id,
    name: skin.name,
    version,
    size: bytes.length,
    sha256: sha256(bytes),
    url: `${origin}/skins/${zipName}`,
  });
  await rm(packDir, { recursive: true, force: true });
  console.log(`${zipName}: ${bytes.length} bytes, ${files.length} files`);
}

await writeFile(
  join(outDir, "index.json"),
  JSON.stringify({ generatedAt: new Date().toISOString(), packs: index }, null, 2) + "\n",
);
console.log(`index.json: ${index.length} packs`);
