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
