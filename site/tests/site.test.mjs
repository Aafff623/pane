import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

import worker from "../worker.js";

const OUT = fileURLToPath(new URL("../out/", import.meta.url));
const p = (rel) => OUT + rel;

function assetFetch(request) {
  const url = new URL(request.url);
  let path = decodeURIComponent(url.pathname).replace(/^\//, "");
  if (path.endsWith("/") || path === "") path += "index.html";
  try {
    const body = readFileSync(p(path));
    return new Response(body, { status: 200, headers: { "content-type": path.endsWith(".html") ? "text/html" : "application/octet-stream" } });
  } catch {
    return new Response("not found", { status: 404 });
  }
}

const env = { ASSETS: { fetch: assetFetch } };
const call = (path, init) => worker.fetch(new Request(`http://localhost${path}`, init), env);

test("static export: pages, languages and assets", () => {
  const en = readFileSync(p("index.html"), "utf8");
  assert.match(en, /<html lang="en"/);
  assert.match(en, /All your AI usage|Every quota window/);
  assert.doesNotMatch(en, /Design Mode|designmode\.app/);

  const zh = readFileSync(p("zh/index.html"), "utf8");
  assert.match(zh, /<html lang="zh-CN"/);
  assert.match(zh, /所有 AI 工具的用量/);

  for (const page of ["demo", "privacy", "terms", "zh/demo", "zh/privacy", "zh/terms"]) {
    assert.ok(existsSync(p(`${page}/index.html`)), `${page} page exists`);
  }

  const seen = new Set();
  for (const page of ["index.html", "zh/index.html", "demo/index.html", "privacy/index.html"]) {
    const html = readFileSync(p(page), "utf8");
    for (const match of html.matchAll(/(?:src|href)="(\/[^"#?]+\.(?:js|css|png|svg|webp|woff2|ico))"/g)) seen.add(match[1]);
  }
  for (const href of seen) assert.ok(existsSync(p(href.replace(/^\//, ""))), `missing ${href}`);
  assert.ok(seen.size > 8);
});

test("worker: framing policy and legacy /en redirects", async () => {
  const home = await call("/");
  assert.equal(home.status, 200);
  assert.equal(home.headers.get("x-frame-options"), "DENY");

  const demo = await call("/pane-demo/index.html");
  assert.equal(demo.status, 200);
  assert.equal(demo.headers.get("x-frame-options"), "SAMEORIGIN");
  assert.match(demo.headers.get("content-security-policy-report-only"), /frame-ancestors 'self'/);

  const legacy = await call("/en/privacy/");
  assert.equal(legacy.status, 301);
  assert.equal(legacy.headers.get("location"), "http://localhost/privacy/");
});

test("worker: download redirect and API fallbacks", async () => {
  const original = globalThis.fetch;
  try {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          tag_name: "v9.9.9",
          html_url: "https://github.com/Aafff623/pane/releases/tag/v9.9.9",
          published_at: "2026-01-01T00:00:00Z",
          assets: [{ name: "Pane-Aafff623_9.9.9_x64-setup.exe", browser_download_url: "https://example.test/Pane-setup.exe", size: 1 }],
        }),
        { status: 200 },
      );
    const map = await call("/api/latest");
    assert.equal(map.status, 200);
    assert.equal((await map.json()).version, "9.9.9");

    const download = await call("/download/windows");
    assert.equal(download.status, 302);
    assert.equal(download.headers.get("location"), "https://example.test/Pane-setup.exe");

    globalThis.fetch = async () => new Response("nope", { status: 500 });
    const fallback = await call("/download/windows");
    assert.equal(fallback.status, 302);
    assert.match(fallback.headers.get("location"), /releases\/latest$/);
  } finally {
    globalThis.fetch = original;
  }
});

test("worker: CN updater manifest uses the Gitee mirror when ready", async () => {
  const original = globalThis.fetch;
  try {
    globalThis.fetch = async (url, init) => {
      const target = String(url);
      if (target.includes("latest.json") && (!init || init.method !== "HEAD")) {
        return new Response(JSON.stringify({ version: "0.12.0", platforms: { "windows-x86_64": { url: "https://github.com/x.exe" } } }), { status: 200 });
      }
      return new Response(null, { status: 200 });
    };
    const res = await call("/update/latest.json", { headers: { "CF-IPCountry": "CN" } });
    assert.equal(res.status, 200);
    const doc = await res.json();
    assert.match(doc.platforms["windows-x86_64"].url, /gitee\.com\/fan-tengda\/pane/);
    assert.equal(res.headers.get("x-pane-manifest-via"), "gitee");
  } finally {
    globalThis.fetch = original;
  }
});

test("worker: skins keep CORS and unknown routes 404", async () => {
  const skins = await call("/skins/index.json");
  assert.equal(skins.status, 200);
  assert.equal(skins.headers.get("access-control-allow-origin"), "*");
  const preflight = await call("/skins/index.json", { method: "OPTIONS" });
  assert.equal(preflight.status, 204);
  const missing = await call("/does-not-exist");
  assert.equal(missing.status, 404);
});

test("facts: version and provider count match the app", () => {
  const rootPkg = JSON.parse(readFileSync(new URL("../../package.json", import.meta.url), "utf8"));
  const facts = readFileSync(new URL("../src/content/facts.ts", import.meta.url), "utf8");
  assert.ok(facts.includes(`version: "${rootPkg.version}"`));
  const catalog = readFileSync(new URL("../../src/providerCatalog.ts", import.meta.url), "utf8");
  const families = new Set([...catalog.matchAll(/familyId:\s*"([^"]+)"/g)].map((m) => m[1]));
  assert.ok(facts.includes(`providers: ${families.size}`), `facts providers should be ${families.size}`);
  const licence = readFileSync(new URL("../LICENSE", import.meta.url), "utf8");
  assert.match(licence, /Commons Clause/);
  assert.match(licence, /shadcnblocks\.com/);
});

test("demo bundle: provenance hashes match the app sources", async () => {
  const { createHash } = await import("node:crypto");
  const provenance = JSON.parse(readFileSync(new URL("../public/pane-demo/source.json", import.meta.url), "utf8"));
  const rootPkg = JSON.parse(readFileSync(new URL("../../package.json", import.meta.url), "utf8"));
  assert.equal(provenance.version, rootPkg.version);
  for (const [file, expected] of Object.entries(provenance.files)) {
    const raw = readFileSync(new URL("../../" + file, import.meta.url), "utf8");
    const digest = createHash("sha256").update(raw.split("\r\n").join("\n")).digest("hex");
    assert.equal(digest, expected, `${file} changed; run node scripts/build-demo.mjs`);
  }
});
