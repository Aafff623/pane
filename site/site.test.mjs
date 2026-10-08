// node --test site/ — the site's invariant locks, zero dependencies:
//   1. Pane landing page keeps its product sections; when the page carries
//      data-i18n keys they stay in step with the parked dictionaries in
//      i18n.js, both directions (a page key with no dictionary entry fails,
//      a dead dictionary key the page doesn't have fails).
//   2. Every local asset index.html references exists under public/.
//   3. GitHub links are exactly Aafff623/pane; the download buttons keep the
//      releases/latest fallback and stay wired for the direct installer.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { LANGS } from "./i18n.js";

const html = await readFile(new URL("./public/index.html", import.meta.url), "utf8");

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
    if (u === "/" || u.startsWith("/#")) continue; // routes, not files
    const clean = u.split("?")[0];
    const p = new URL("./public" + clean, import.meta.url);
    if (!existsSync(p)) missing.push(u);
  }
  assert.deepEqual(missing, []);
});

test("links: GitHub goes to Aafff623/pane, downloads via releases/latest", async () => {
  const gh = [...html.matchAll(/https:\/\/github\.com\/[^"\s<)]*/g)].map((m) => m[0]);
  assert.ok(gh.length >= 4, "expected repo, releases and issues links");
  for (const u of gh) assert.ok(u.startsWith("https://github.com/Aafff623/pane"), u);
  assert.ok(gh.includes("https://github.com/Aafff623/pane/releases/latest"), "download link");
  assert.ok(!html.includes("usemagpie"), "no Magpie leftovers");
  // Every download button upgrades to the newest Windows installer at runtime
  // (site.js); the releases page stays as the no-JS / blocked-API fallback.
  const wired = [...html.matchAll(/<a[^>]*data-latest-installer[^>]*>/g)];
  assert.equal(wired.length, 3, "download buttons carry the wiring");
  for (const [tag] of wired) assert.ok(tag.includes("releases/latest"), tag);
  const js = await readFile(new URL("./public/site.js", import.meta.url), "utf8");
  assert.ok(js.includes("data-latest-installer"), "site.js resolves the installer");
  assert.ok(js.includes("browser_download_url"), "resolver reads the API asset url");
});
