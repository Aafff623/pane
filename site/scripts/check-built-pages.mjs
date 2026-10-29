import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const app = join(root, ".next/server/app");
const origin = "https://designmode.app";

function htmlFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory()
      ? htmlFiles(path)
      : path.endsWith(".html")
        ? [path]
        : [];
  });
}

function assetExists(pathname) {
  return pathname.startsWith("/_next/")
    ? existsSync(join(root, ".next", pathname.slice("/_next/".length)))
    : existsSync(join(root, "public", pathname));
}

const pages = htmlFiles(app);
assert(pages.length > 0, "Run npm run build before checking built pages");
const sitemap = readFileSync(join(app, "sitemap.xml.body"), "utf8");
const locations = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map(
  (match) => match[1],
);
assert.equal(
  new Set(locations).size,
  locations.length,
  "Duplicate sitemap URLs",
);
assert(
  locations.includes(`${origin}/changelog`),
  "Published changelog route must stay discoverable",
);

for (const location of locations) {
  const url = new URL(location);
  assert.equal(url.origin, origin, `Unexpected sitemap origin: ${location}`);
  const pathname = url.pathname.replace(/\/$/, "") || "/index";
  const file = join(app, `${pathname}.html`);
  assert(existsSync(file), `Missing sitemap page: ${location}`);
  const html = readFileSync(file, "utf8");
  assert(
    !/<meta[^>]*name="robots"[^>]*content="[^"]*noindex/.test(html),
    `Sitemap includes noindex page: ${location}`,
  );
  const canonical = html.match(
    /<link\b[^>]*rel="canonical"[^>]*href="([^"]+)"/,
  );
  assert(canonical, `Missing canonical URL: ${location}`);
  assert.equal(
    new URL(canonical[1]).href,
    url.href,
    `Incorrect canonical URL: ${location}`,
  );
}

let images = 0;
for (const file of pages) {
  const html = readFileSync(file, "utf8");
  for (const [, value] of html.matchAll(/<img\b[^>]*\bsrc="([^"]+)"/g)) {
    const url = new URL(value.replaceAll("&amp;", "&"), origin);
    if (url.origin !== origin) continue;
    const asset =
      url.pathname === "/_next/image"
        ? url.searchParams.get("url")
        : url.pathname;
    if (!asset?.startsWith("/") || asset.startsWith("//")) continue;
    assert(
      assetExists(decodeURIComponent(asset)),
      `Missing image ${asset} in ${file}`,
    );
    images++;
  }
}

console.log(
  `Built-page checks passed: ${pages.length} HTML files, ${locations.length} sitemap routes, ${images} local image references.`,
);
