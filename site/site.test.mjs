// node --test site/ — the site's invariant locks, zero dependencies:
//   1. i18n parity, both directions: a key on the page missing from a
//      dictionary fails; a dead dictionary key the page doesn't have fails.
//   2. Every T( call passes one argument per language (T's fallback is
//      silent — a missing argument would show Chinese to en readers and
//      never error).
//   3. Every local asset index.html references exists under public/.
//   4. GitHub links are exactly Aafff623/pane, downloads via releases/latest.
//   5. The worker's language negotiation (cookie / Accept-Language / source).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import worker, { preferred } from "./worker.js";
import { LANGS } from "./i18n.js";

const html = await readFile(new URL("./public/index.html", import.meta.url), "utf8");

test("i18n parity: every key the page marks is in each dictionary, and no other", () => {
  const keys = new Set();
  for (const [, k] of html.matchAll(/data-i18n="([^"]+)"/g)) keys.add(k);
  for (const [, pairs] of html.matchAll(/data-i18n-attr="([^"]+)"/g))
    for (const p of pairs.split(",")) keys.add(p.split(":")[1]);
  assert.ok(keys.size > 20, "expected a page full of marked strings");
  for (const [lang, { dict }] of Object.entries(LANGS)) {
    assert.deepEqual([...keys].filter((k) => !(k in dict)), [], `${lang} lacks`);
    assert.deepEqual(Object.keys(dict).filter((k) => !keys.has(k)), [], `${lang} has unused`);
  }
});

// callArgs walks src with a state machine (strings, line and block
// comments) and, for every T( outside comments/strings, counts the call's
// top-level arguments — commas at depth zero, past nested () [] {}.
function callArgs(src) {
  const out = [];
  let quote = null, line = false, block = false;
  for (let i = 0; i < src.length; i++) {
    const c = src[i], n = src[i + 1];
    if (line) { if (c === "\n") line = false; continue; }
    if (block) { if (c === "*" && n === "/") { block = false; i++; } continue; }
    if (quote) {
      if (c === "\\") i++;
      else if (c === quote) quote = null;
      continue;
    }
    if (c === "/" && n === "/") { line = true; i++; continue; }
    if (c === "/" && n === "*") { block = true; i++; continue; }
    if (c === '"' || c === "'" || c === "`") { quote = c; continue; }
    if (c === "T" && n === "(" && !/[\w$]/.test(src[i - 1] || "")) {
      let depth = 0, args = 1;
      for (let j = i + 2; j < src.length; j++) {
        const d = src[j], e = src[j + 1];
        if (d === "/" && e === "/") { while (j < src.length && src[j] !== "\n") j++; continue; }
        if (d === "/" && e === "*") { j += 2; while (j < src.length && !(src[j] === "*" && src[j + 1] === "/")) j++; j++; continue; }
        if (d === '"' || d === "'" || d === "`") { const q = d; j++; while (j < src.length && src[j] !== q) { if (src[j] === "\\") j++; j++; } continue; }
        if (d === "(" || d === "[" || d === "{") depth++;
        else if (d === ")" && depth === 0) { out.push({ at: i, args }); break; }
        else if (d === ")" || d === "]" || d === "}") depth--;
        else if (d === "," && depth === 0) args++;
        if (j === src.length - 1) throw new Error(`unbalanced T( at offset ${i}`);
      }
      i++;
    }
  }
  return out;
}

test("T( calls pass both languages (no silent fallback to Chinese)", () => {
  const calls = callArgs(html);
  assert.ok(calls.length > 0, "expected at least one T( call");
  for (const { at, args } of calls)
    assert.ok(args >= 1 + Object.keys(LANGS).length, `T( at offset ${at} has ${args} argument(s)`);
});

test("assets: every local file index.html references exists", () => {
  const missing = [];
  for (const [, u] of html.matchAll(/(?:src|href)="(\/[^"]*)"/g)) {
    if (u === "/" || u === "/en/" || u.startsWith("/#")) continue; // routes, not files
    const p = new URL("./public" + u, import.meta.url);
    if (!existsSync(p)) missing.push(u);
  }
  assert.deepEqual(missing, []);
});

test("links: GitHub goes to Aafff623/pane, downloads to releases/latest", () => {
  const gh = [...html.matchAll(/https:\/\/github\.com\/[^"\s<)]*/g)].map((m) => m[0]);
  assert.ok(gh.length >= 4, "expected repo, releases and issues links");
  for (const u of gh) assert.ok(u.startsWith("https://github.com/Aafff623/pane"), u);
  assert.ok(gh.includes("https://github.com/Aafff623/pane/releases/latest"), "download link");
  assert.ok(!html.includes("usemagpie"), "no Magpie leftovers");
});

test("home: / sends a browser that prefers English to /en/, until a language is picked", () => {
  const env = { ASSETS: { fetch: async () => new Response("<html lang=\"zh-CN\">", { headers: { "Content-Type": "text/html" } }) } };
  const to = async (headers) => {
    const res = await worker.fetch(new Request("https://pane.local/", { headers }), env);
    assert.match(res.headers.get("Vary") || "", /Accept-Language/);
    return res.status === 302 ? res.headers.get("Location") : res.status;
  };
  return (async () => {
    assert.equal(await to({}), 200);
    assert.equal(await to({ "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8" }), 200);
    assert.equal(await to({ "Accept-Language": "en-US,en;q=0.9,zh;q=0.8" }), "/en/");
    assert.equal(await to({ "Accept-Language": "fr-FR,en;q=0.5,zh;q=0.4" }), "/en/");
    assert.equal(await to({ "Accept-Language": "en;q=0.2,zh;q=0.8" }), 200);
    assert.equal(await to({ "Accept-Language": "en-US", Cookie: "a=b; lang=zh" }), 200);
    assert.equal(await to({ "Accept-Language": "zh-CN", Cookie: "lang=en" }), "/en/");
    assert.equal(await to({ Cookie: "lang=fr" }), 200); // a language the site lacks: the source
  })();
});

test("preferred: cookie wins, then Accept-Language, then the source language", () => {
  const p = (headers) => preferred(new Request("https://pane.local/", { headers }));
  assert.equal(p({}), "zh");
  assert.equal(p({ "Accept-Language": "en-GB" }), "en");
  assert.equal(p({ "Accept-Language": "en", Cookie: "lang=zh" }), "zh");
});

test("lang route: /en redirects to /en/", async () => {
  const env = { ASSETS: { fetch: async () => new Response("unreachable") } };
  const res = await worker.fetch(new Request("https://pane.local/en"), env);
  assert.equal(res.status, 301);
  assert.equal(new URL(res.headers.get("Location")).pathname, "/en/");
});
