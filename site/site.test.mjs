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
const usageFlowScript = await readFile(new URL("./public/usage-flow.js", import.meta.url), "utf8");
const RELEASES_PAGE = "https://github.com/Aafff623/pane/releases/latest";

test("site content: Pane landing page includes the product sections", () => {
  const keys = new Set();
  for (const [, k] of html.matchAll(/data-i18n="([^"]+)"/g)) keys.add(k);
  for (const [, pairs] of html.matchAll(/data-i18n-attr="([^"]+)"/g))
    for (const p of pairs.split(",")) keys.add(p.split(":")[1]);
  assert.match(html, /id="demo"/);
  assert.match(html, /id="features"/);
  assert.match(html, /id="faq"/);
  // The landing page currently carries no data-i18n attributes (English is
  // applied by site.js), so the vacuous early return was hiding every
  // dictionary gap. Lock the keys site.js actually calls instead.
  const used = new Set();
  for (const [, key] of siteScript.matchAll(/setText\(\s*['"][^'"]+['"]\s*,\s*['"]([^'"]+)['"]/g)) used.add(key);
  for (const [, list] of siteScript.matchAll(/setTexts\(\s*['"][^'"]+['"]\s*,\s*\[([^\]]+)\]/g)) {
    for (const [, key] of list.matchAll(/['"]([^'"]+)['"]/g)) used.add(key);
  }
  // The annotation table is [selector, key] pairs consumed in a loop.
  for (const [, key] of siteScript.matchAll(/\[['"]\.[^'"]+['"],\s*['"]([A-Za-z0-9_.]+)['"]\]/g)) used.add(key);
  assert.ok(used.size >= 40, `expected site.js to use its dictionary, saw ${used.size}`);
  // `en.key` property reads (usage-flow, meta, FAQ helpers) are real uses too.
  const direct = new Set();
  for (const source of [siteScript, usageFlowScript]) {
    for (const [, key] of source.matchAll(/\ben\.([A-Za-z0-9_]+)/g)) direct.add(key);
  }
  for (const [lang, { dict }] of Object.entries(LANGS)) {
    const missing = [...used].filter((key) => !(key in dict));
    assert.deepEqual(missing, [], `${lang} lacks site.js keys`);
    if (keys.size) {
      assert.deepEqual([...keys].filter((k) => !(k in dict)), [], `${lang} lacks data-i18n keys`);
    }
    // A key nobody reads is stale copy. FAQ keys are assembled from group/item
    // indices, so they are exempt from the flat scan but still checked above
    // through the `used` set and the structure test below.
    const unused = Object.keys(dict).filter(
      (key) => !used.has(key) && !direct.has(key) && !key.startsWith("faq."),
    );
    assert.deepEqual(unused, [], `${lang} has unused dictionary keys`);
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

test("assets: the site's lazy effect imports resolve to local JavaScript", async () => {
  const pending = [new URL('./public/site.js', import.meta.url)];
  const visited = new Set();
  while (pending.length) {
    const file = pending.pop();
    if (visited.has(file.href)) continue;
    visited.add(file.href);
    const source = await readFile(file, 'utf8');
    for (const [, specifier] of source.matchAll(/(?:from\s*|import\s*\()\s*['"]([^'"]+)['"]/g)) {
      const dependency = specifier.startsWith('/')
        ? new URL(`./public${specifier.split('?')[0]}`, import.meta.url)
        : new URL(specifier.split('?')[0], file);
      assert.ok(existsSync(dependency), `${file.pathname} imports missing ${specifier}`);
      pending.push(dependency);
    }
  }
});

test("links: GitHub goes to Aafff623/pane, download buttons use the edge route", () => {
  const gh = [...html.matchAll(/https:\/\/github\.com\/[^"\s<)]*/g)].map((m) => m[0]);
  assert.ok(gh.length >= 4, "expected repo, releases and issues links");
  for (const u of gh) assert.ok(u.startsWith("https://github.com/Aafff623/pane"), u);
  assert.ok(!html.includes("usemagpie"), "no Magpie leftovers");
  // No client script and no CORS: the button carries a stable same-origin
  // route and the worker does the GitHub lookup.
  const wired = [...html.matchAll(/<a[^>]*data-latest-installer[^>]*>/g)];
  assert.equal(wired.length, 2, "primary download buttons");
  for (const [tag] of wired) assert.ok(tag.includes('href="/download/windows"'), tag);
  assert.match(html, /data-platform="windows" href="\/download\/windows"/, "the Windows card keeps its edge route");
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

// 4b. The signal cards (#88) are the one section adapted from the oil-ui-pro
//     prototypes. They must stay on the site's token palette and keep naming
//     mechanisms the app actually ships, not decorative mock data.
test("signals: four instrument cards use design tokens and real endpoints", async () => {
  const start = html.indexOf('id="signals"');
  const end = html.indexOf('<section id="design"');
  assert.ok(start > 0 && end > start, "signals section sits before design");
  const section = html.slice(start, end);
  assert.equal([...section.matchAll(/<article class="signal-card/g)].length, 4);
  assert.doesNotMatch(section, /#[0-9a-fA-F]{3,8}\b/, "no raw colours in the section");
  assert.match(section, /openusage\.limits\.v1/);
  assert.match(section, /data-number="12400000"/);
  const css = await readFile(new URL("./public/site.css", import.meta.url), "utf8");
  const block = css.slice(css.indexOf("/* Signals (#88)"));
  assert.ok(block.length > 1000, "signal styles exist");
  assert.doesNotMatch(
    block.slice(0, block.indexOf("@media(prefers-reduced-motion")),
    /#[0-9a-fA-F]{3,8}\b/,
    "signal styles use tokens only",
  );
});

// 4c. The comic preview (#89) is opt-in: it must stay behind the
//     data-site-skin attribute, keep its own tokens, and never restyle a
//     native visitor's page.
test("comic skin: opt-in attribute, scoped styles, persisted toggle", async () => {
  const skinScript = await readFile(new URL("./public/site-skin.js", import.meta.url), "utf8");
  assert.match(html, /id="site-skin"/);
  assert.match(html, /site-skin\.js/);
  assert.match(skinScript, /pane-site-skin/);
  assert.match(skinScript, /data-site-skin/);
  const css = await readFile(new URL("./public/site.css", import.meta.url), "utf8");
  const start = css.indexOf("/* Comic-pixel site skin");
  assert.ok(start > 0, "comic block exists");
  const block = css.slice(start);
  assert.match(block, /:root\[data-site-skin=comic-pixel\]/);
  assert.match(block, /--comic-shadow/);
  assert.doesNotMatch(css.slice(0, start), /comic-shadow|comic-border/, "native styles never use comic tokens");
});

// 4c2. The demo iframe is the heaviest request on the page. It must start
//      without a src and only receive one when the demo section approaches;
//      `loading="lazy"` alone is too eager on wide viewports (#85).
test("demo: iframe starts blank and loads near the section", () => {
  const tag = html.match(/<iframe id="pane-demo"[^>]*>/)?.[0];
  assert.ok(tag, "demo iframe present");
  assert.match(tag, /loading="lazy"/);
  assert.doesNotMatch(tag, /\bsrc=/, "no first-paint demo request");
  assert.match(siteScript, /rootMargin:'700px 0px'/);
  assert.match(siteScript, /demoBooted/);
});

// 4d. The motion pack (#86) is trigger-only: one count-up per figure, a
//     frosted header after the top, and a reduced-motion end state for every
//     entrance. Ambient looping on content would violate the design contract.
test("motion: count-up, frosted header and reduced-motion end states", async () => {
  const css = await readFile(new URL("./public/site.css", import.meta.url), "utf8");
  assert.match(siteScript, /Math\.pow\(1-t,3\)/, "count-up easing");
  assert.match(siteScript, /1500|1400/, "count-up duration");
  assert.match(siteScript, /is-revealed/, "text reveal class");
  assert.match(siteScript, /indexPop/, "section index pop");
  assert.match(css, /\.site-header\.is-scrolled/, "frosted header");
  assert.match(css, /@keyframes label-type/, "typewriter label");
  assert.match(css, /prefers-reduced-motion:reduce\)\{\.motion-ready \.reveal-text/, "reveal has a reduced-motion end state");
  assert.equal([...html.matchAll(/class="reveal-text"/g)].length, 2, "two headline reveals");
});

// 5. The hero copy carries two numbers the app owns: the release version
//    (package.json) and the provider count (src/providerCatalog.ts). Both
//    drifted before (v0.4 / 43 against 0.5.0 / 66); pin them here so the next
//    catalog expansion or version bump fails the suite instead of shipping
//    stale marketing copy. Every occurrence is checked — the zh body says the
//    count in two places, the hero stat block once, and the served English
//    dictionary once — a single stale spot must fail the suite.
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
  assert.match(html, new RegExp(`<b[^>]*>${providerCount}</b><span>服务商</span>`), "hero stats show the catalog size");
  const zhCounts = [...html.matchAll(/(\d+) 个服务商/g)].map((m) => Number(m[1]));
  assert.ok(zhCounts.length >= 2, "zh body mentions the provider count");
  assert.deepEqual([...new Set(zhCounts)], [providerCount], `every zh mention says ${providerCount}, saw ${zhCounts}`);
  assert.ok(LANGS.en.dict.heroMetaRight.endsWith(`v${pkg.version}`), "English hero shows the current version");
  assert.ok(html.includes(`data-latest-version>v${pkg.version}</b>`), "release fallback shows the package version");
  assert.match(html, /download-routing\.js/, "platform routing is wired");
  assert.match(html, /data-platform="windows"/);
  assert.match(html, /data-platform="macos"/);
  const servedEnglish = await import("./public/i18n.js");
  assert.deepEqual(servedEnglish.en, LANGS.en.dict, "Worker and browser English dictionaries stay in sync");
  const enCopy = `${LANGS.en.dict.providersCopy} ${LANGS.en.dict.ecosystemIntro}`;
  const enCounts = [...enCopy.matchAll(/\b(\d+) providers\b/g)].map((m) => Number(m[1]));
  assert.ok(enCounts.length >= 2, "both English provider descriptions mention the catalog size");
  assert.deepEqual([...new Set(enCounts)], [providerCount], `every en mention says ${providerCount}, saw ${enCounts}`);
});

test("English downloads: both Windows labels have a translation target without replacing their icons", () => {
  const windowsLinks = [...html.matchAll(/<a\b[^>]*>([\s\S]*?)<\/a>/g)]
    .map((match) => match[1])
    .filter((body) => body.includes("windows-logo.svg") && body.includes("data-download-label"));
  assert.equal(windowsLinks.length, 2);
  for (const body of windowsLinks) assert.match(body, /<span data-download-label>下载 Windows 版<\/span>/);
  assert.match(siteScript, /setTexts\('\[data-download-label\]', \['heroDownload','heroDownload'\]\)/);
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
            { name: "Pane-Aafff623_9.9.9_aarch64.dmg", browser_download_url: "https://example.test/mac.dmg" },
            { name: "Pane-Aafff623_9.9.9_x64-setup.exe", browser_download_url: "https://example.test/win.exe" },
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

test("api/latest: version and asset list for platform routing", async () => {
  const env = { ASSETS: { fetch: async () => new Response("assets") } };
  const realFetch = globalThis.fetch;
  try {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          tag_name: "v9.9.9",
          html_url: "https://example.test/releases/tag/v9.9.9",
          published_at: "2026-10-26T00:00:00Z",
          assets: [
            { name: "Pane-Aafff623_9.9.9_x64-setup.exe", browser_download_url: "https://example.test/win.exe", size: 100 },
            { name: "Pane-Aafff623_9.9.9_aarch64.dmg", browser_download_url: "https://example.test/mac.dmg", size: 200 },
          ],
        }),
        { status: 200 },
      );
    const res = await worker.fetch(new Request("https://pane.local/api/latest"), env);
    assert.equal(res.status, 200);
    assert.match(res.headers.get("cache-control"), /max-age=300/);
    const body = await res.json();
    assert.equal(body.version, "9.9.9");
    assert.equal(body.tag, "v9.9.9");
    assert.equal(body.assets.length, 2);
    assert.equal(body.assets[1].size, 200);

    globalThis.fetch = async () => new Response("rate limited", { status: 403 });
    const failed = await worker.fetch(new Request("https://pane.local/api/latest"), env);
    assert.equal(failed.status, 502);
    assert.equal((await failed.json()).error, "unavailable");
    assert.equal(failed.headers.get("cache-control"), "no-store");
  } finally {
    globalThis.fetch = realFetch;
  }
});

test("update/latest.json: CN clients get the Gitee-mirrored installer", async () => {
  const env = { ASSETS: { fetch: async () => new Response("assets") } };
  const manifest = {
    version: "9.9.9",
    platforms: {
      "windows-x86_64": { url: "https://github.com/x/win.exe", signature: "SIG" },
      "darwin-aarch64": { url: "https://github.com/x/mac.dmg", signature: "SIG2" },
    },
  };
  const at = async (headers = {}, search = "") =>
    worker.fetch(new Request(`https://pane.local/update/latest.json${search}`, { headers }), env);
  const realFetch = globalThis.fetch;
  try {
    const seen = [];
    const stub = (headStatus) => async (input, init = {}) => {
      const url = String(input);
      seen.push(`${init.method ?? "GET"} ${url}`);
      if (url.includes("releases/latest/download/latest.json")) {
        return new Response(JSON.stringify(manifest), { status: 200 });
      }
      if (url.startsWith("https://gitee.com/")) return new Response(null, { status: headStatus });
      throw new Error(`unexpected fetch ${url}`);
    };

    globalThis.fetch = stub(200);
    const cn = await at({ "CF-IPCountry": "CN" });
    assert.equal(cn.status, 200);
    assert.equal(cn.headers.get("x-pane-manifest-via"), "gitee");
    const body = await cn.json();
    assert.equal(
      body.platforms["windows-x86_64"].url,
      "https://gitee.com/fan-tengda/pane/releases/download/v9.9.9/Pane-Aafff623_9.9.9_x64-setup.exe",
      "windows URL rewritten to the mirror",
    );
    assert.equal(body.platforms["windows-x86_64"].signature, "SIG", "signature untouched");
    assert.equal(body.platforms["darwin-aarch64"].url, "https://github.com/x/mac.dmg", "other platforms untouched");
    assert.ok(seen.some((s) => s.startsWith("HEAD https://gitee.com/")), "mirror probed with HEAD");

    globalThis.fetch = stub(200);
    const us = await at({ "CF-IPCountry": "US" });
    assert.equal(us.headers.get("x-pane-manifest-via"), "github");
    assert.equal((await us.json()).platforms["windows-x86_64"].url, "https://github.com/x/win.exe");

    globalThis.fetch = stub(404);
    const missing = await at({ "CF-IPCountry": "CN" });
    assert.equal(missing.headers.get("x-pane-manifest-via"), "github", "missing mirror falls back to GitHub");
    assert.equal((await missing.json()).platforms["windows-x86_64"].url, "https://github.com/x/win.exe");

    globalThis.fetch = stub(404);
    const forced = await at({ "CF-IPCountry": "US" }, "?source=gitee");
    assert.equal(forced.headers.get("x-pane-manifest-via"), "github", "override still requires a ready mirror");

    globalThis.fetch = stub(200);
    const forcedOff = await at({ "CF-IPCountry": "CN" }, "?source=github");
    assert.equal(forcedOff.headers.get("x-pane-manifest-via"), "github", "source=github disables the rewrite");

    globalThis.fetch = async () => new Response("nope", { status: 500 });
    assert.equal((await at({ "CF-IPCountry": "CN" })).status, 502, "upstream failure surfaces as 502");
  } finally {
    globalThis.fetch = realFetch;
  }
});


test("basic contracts: security headers, 404 fallback, robots/sitemap/OG/skip-link", async () => {
  const { readFileSync } = await import("node:fs");
  for (const file of ["public/robots.txt", "public/sitemap.xml", "public/404.html", "public/og-cover.png"]) {
    assert.ok(existsSync(new URL(`./${file}`, import.meta.url)), `${file} exists`);
  }
  const page = readFileSync(new URL("./public/index.html", import.meta.url), "utf8");
  assert.match(page, /class="skip-link" href="#main-content"/);
  assert.match(page, /<main id="main-content">/);
  assert.match(page, /og:image:width" content="1200"/);
  assert.match(page, /og:image:height" content="630"/);
  assert.match(page, /twitter:card" content="summary_large_image"/);
  assert.match(page, /id="boundaries"/);
  assert.match(page, /href="\/privacy.html"/);
  assert.match(page, /href="\/terms.html"/);
  assert.match(page, /not affiliated with any AI provider/);
  assert.match(page, /<iframe id="pane-demo" loading="lazy"/);
  const env = (body = "ok", status = 200) => ({
    ASSETS: { fetch: async (request) => {
      const path = new URL(request.url).pathname;
      if (status === 404 && path !== "/404.html") return new Response(body, { status: 404 });
      if (status === 404) return new Response("<h1>这个页面不存在</h1>", { status: 200, headers: { "content-type": "text/html" } });
      return new Response(body, { status, headers: { "content-type": "text/plain" } });
    } },
  });
  const ok = await worker.fetch(new Request("https://pane.threetwoa.live/robots.txt"), env());
  assert.equal(ok.headers.get("x-content-type-options"), "nosniff");
  assert.equal(ok.headers.get("x-frame-options"), "DENY");
  assert.ok(ok.headers.get("content-security-policy-report-only")?.includes("default-src 'self'"));
  assert.equal(ok.headers.get("referrer-policy"), "strict-origin-when-cross-origin");
  const missing = await worker.fetch(new Request("https://pane.threetwoa.live/nope"), env("nf", 404));
  assert.equal(missing.status, 404);
  assert.match(await missing.text(), /这个页面不存在/);
});
