// node --test site/ — the site's invariant locks, zero dependencies:
//   1. Pane landing page keeps its product sections; when the page carries
//      data-i18n keys they stay in step with the parked dictionaries in
//      i18n.js, both directions (a page key with no dictionary entry fails,
//      a dead dictionary key the page doesn't have fails).
//   2. Every local asset index.html references exists under public/.
//   3. GitHub links are exactly Aafff623/pane; every download button points
//      at the worker's stable /download/windows route.
//   4. /download/windows resolves the newest Windows installer at the edge,
//      and falls back to the releases page whenever the API cannot answer.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { createHash } from "node:crypto";
import worker from "./worker.js";
import { LANGS } from "./i18n.js";

const html = await readFile(new URL("./public/index.html", import.meta.url), "utf8");
const siteScript = await readFile(new URL("./public/site.js", import.meta.url), "utf8");
const RELEASES_PAGE = "https://github.com/Aafff623/pane/releases/latest";

test("site content: Pane landing page includes the product sections", () => {
  const keys = new Set();
  for (const [, k] of html.matchAll(/data-i18n="([^"]+)"/g)) keys.add(k);
  for (const [, pairs] of html.matchAll(/data-i18n-attr="([^"]+)"/g))
    for (const p of pairs.split(",")) keys.add(p.split(":")[1]);
  assert.match(html, /id="demo"/);
  assert.match(html, /id="features"/);
  assert.match(html, /id="faq"/);
  if (keys.size === 0) return;
  for (const [lang, { dict }] of Object.entries(LANGS)) {
    assert.deepEqual([...keys].filter((k) => !(k in dict)), [], `${lang} lacks`);
    assert.deepEqual(Object.keys(dict).filter((k) => !keys.has(k)), [], `${lang} has unused`);
  }
});

test("assets: every local file index.html references exists", () => {
  const missing = [];
  for (const [, u] of html.matchAll(/(?:src|href)="(\/[^"]*)"/g)) {
    if (u === "/" || u.startsWith("/#") || u === "/download/windows") continue; // routes, not files
    const clean = u.split("?")[0];
    const p = new URL("./public" + clean, import.meta.url);
    if (!existsSync(p)) missing.push(u);
  }
  assert.deepEqual(missing, []);
});

test("links: GitHub goes to Aafff623/pane, download buttons use the edge route", () => {
  const gh = [...html.matchAll(/https:\/\/github\.com\/[^"\s<)]*/g)].map((m) => m[0]);
  assert.ok(gh.length >= 4, "expected repo, releases and issues links");
  for (const u of gh) assert.ok(u.startsWith("https://github.com/Aafff623/pane"), u);
  assert.ok(!html.includes("usemagpie"), "no Magpie leftovers");
  // No client script and no CORS: the button carries a stable same-origin
  // route and the worker does the GitHub lookup.
  const wired = [...html.matchAll(/<a[^>]*data-latest-installer[^>]*>/g)];
  assert.equal(wired.length, 3, "download buttons");
  for (const [tag] of wired) assert.ok(tag.includes('href="/download/windows"'), tag);
});

test("theme: host and embedded panel themes stay isolated", () => {
  assert.match(siteScript, /let demoTheme='dark'/);
  assert.doesNotMatch(siteScript, /settingsFrame\.contentWindow\?\.postMessage\(\{type:'pane-demo',action:'theme'\}/);
  assert.match(siteScript, /transport\(["']theme["']\)/);
});

test("demo: bundled frontend matches the current app sources", async () => {
  const provenance = JSON.parse(await readFile(new URL("./public/demo/source.json", import.meta.url), "utf8"));
  for (const [file, expected] of Object.entries(provenance.files)) {
    const current = createHash("sha256").update((await readFile(new URL(`../${file}`, import.meta.url), "utf8")).replaceAll("\r\n", "\n")).digest("hex");
    assert.equal(current, expected, `${file} changed; run node site/scripts/build-demo.mjs`);
  }
});

// 5. The hero copy carries two numbers the app owns: the release version
//    (package.json) and the provider count (src/providerCatalog.ts). Both
//    drifted before (v0.4 / 43 against 0.5.0 / 66); pin them here so the next
//    catalog expansion or version bump fails the suite instead of shipping
//    stale marketing copy.
test("copy: hero numbers match package.json and the provider catalog", async () => {
  const pkg = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
  );
  const catalog = await readFile(
    new URL("../src/providerCatalog.ts", import.meta.url),
    "utf8",
  );
  const providerCount = [...catalog.matchAll(/familyId:\s*"[^"]+"/g)].length;
  assert.ok(providerCount > 0, "catalog entries found");
  assert.ok(html.includes(`桌面伴侣 · v${pkg.version}`), `hero meta shows v${pkg.version}`);
  assert.match(html, new RegExp(`<b>${providerCount}</b><span>服务商</span>`), "hero stats show the catalog size");
  assert.ok(html.includes(`${providerCount} 个服务商`), "provider section names the catalog size");
});

// 6. The FAQ is a grouped accordion (Ruixen-style): three topic groups of
//    three rows, every row carrying an answer body wrapper — the wrapper is
//    what the grid-template-rows height animation and the visibility-based
//    accessibility cut-out both depend on, so a row without one is a defect.
test("faq: three groups of three rows, each row with an answer body", () => {
  const count = (re) => [...html.matchAll(re)].length;
  assert.equal(count(/class="faq-group"/g), 3, "faq groups");
  assert.equal(count(/class="faq-item"/g), 9, "faq rows");
  assert.equal(count(/class="faq-body"/g), 9, "faq answer bodies");
  assert.match(html, /class="faq-contact"/, "contact link block");
});

test("copy: Chinese and English refresh defaults match the app", async () => {
  const app = await readFile(new URL("../src/main.ts", import.meta.url), "utf8");
  const minutes = Number(app.match(/let config: Config = \{\s*refreshMinutes:\s*(\d+)/)?.[1]);
  assert.equal(minutes, 5, "update both language copies if the default changes");
  assert.ok(html.includes(`默认每 ${minutes} 分钟刷新`));
  const bridge = await readFile(new URL("./demo/bridge.ts", import.meta.url), "utf8");
  assert.match(bridge, new RegExp(`refreshMinutes: ${minutes},`), "demo uses the app default");
  assert.match(LANGS.en.dict.runtimeSettingsCopy, /every five minutes by default/);
  assert.equal(await readFile(new URL("./public/i18n.js", import.meta.url), "utf8"),
    await readFile(new URL("./i18n.js", import.meta.url), "utf8"), "served dictionary matches source");
});

test("English routes: both path variants serve index and preserve query", async () => {
  for (const path of ["/en", "/en/", "/en/?view=demo"]) {
    let assetUrl;
    const response = await worker.fetch(new Request(`https://pane.local${path}`), {
      ASSETS: { fetch: async (req) => { assetUrl = req.url; return new Response("index", { status: 200 }); } },
    });
    assert.equal(response.status, 200);
    assert.equal(new URL(assetUrl).pathname, "/");
    assert.equal(new URL(assetUrl).search, new URL(`https://pane.local${path}`).search);
  }
});

test("metadata: English search and share copy is present before JavaScript runs", async () => {
  for (const path of ["/en", "/en/", "/?lang=en"]) {
    const res = await worker.fetch(new Request(`https://pane.local${path}`), {
      ASSETS: { fetch: async () => new Response(html, { headers: { "content-type": "text/html; charset=utf-8", "etag": "original" } }) },
    });
    const body = await res.text();
    assert.ok(body.includes(`<title>${LANGS.en.dict.metaTitle}</title>`));
    assert.ok(body.includes(`name="description" content="${LANGS.en.dict.metaDescription}"`));
    assert.ok(body.includes(`property="og:description" content="${LANGS.en.dict.metaDescription}"`));
    assert.ok(body.includes(`name="twitter:description" content="${LANGS.en.dict.metaDescription}"`));
    assert.match(body, /<html lang="en"/);
    assert.ok(body.includes('rel="canonical" href="https://pane.threetwoa.live/en/"'));
    assert.ok(body.includes('property="og:url" content="https://pane.threetwoa.live/en/"'));
    assert.ok(body.includes('property="og:locale" content="en_US"'));
    assert.equal(res.headers.get("etag"), null, "rewritten body has no stale asset validator");
    assert.equal(res.headers.get("content-language"), "en");
  }
  const root = await worker.fetch(new Request("https://pane.local/"), {
    ASSETS: { fetch: async () => new Response(html, { headers: { "content-type": "text/html" } }) },
  });
  assert.equal(await root.text(), html, "Chinese source stays intact");
});

// 7. The wordmark footer sits outside the link row (the link row's English
//    copy is applied by a positional selector, so an extra <a> inside it
//    would misalign) and keeps both the giant wordmark and the link row.
test("footer: wordmark block sits beside the untouched link row", () => {
  assert.match(html, /id="wordmark-footer"/);
  assert.match(html, /class="wf-word"/);
  assert.match(html, /class="wrap footer-content"/);
  const footer = html.slice(html.indexOf('<footer'));
  const wordmarkAt = footer.indexOf('wordmark-footer');
  const linksAt = footer.indexOf('footer-content');
  assert.ok(wordmarkAt !== -1 && linksAt !== -1 && wordmarkAt < linksAt, "wordmark precedes the link row");
});

test("download/windows: newest installer, else the releases page", async () => {
  const env = { ASSETS: { fetch: async () => new Response("assets") } };
  const at = async () => {
    const res = await worker.fetch(new Request("https://pane.local/download/windows"), env);
    assert.equal(res.status, 302);
    return res.headers.get("Location");
  };
  const realFetch = globalThis.fetch;
  try {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          assets: [
            { name: "Pane_9.9.9_aarch64.dmg", browser_download_url: "https://example.test/mac.dmg" },
            { name: "Pane_9.9.9_x64-setup.exe", browser_download_url: "https://example.test/win.exe" },
          ],
        }),
        { status: 200 },
      );
    assert.equal(await at(), "https://example.test/win.exe", "picks the Windows installer");

    globalThis.fetch = async () => new Response("rate limited", { status: 403 });
    assert.equal(await at(), RELEASES_PAGE, "http failure falls back");

    globalThis.fetch = async () => {
      throw new Error("offline");
    };
    assert.equal(await at(), RELEASES_PAGE, "network failure falls back");
  } finally {
    globalThis.fetch = realFetch;
  }
});
