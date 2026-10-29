#!/usr/bin/env node
// site/scripts/sync-site-version.mjs — one command keeps the two numbers the
// marketing copy must not hand-maintain in sync with their sources:
//
//   version       package.json
//   providerCount src/providerCatalog.ts (the cards a visitor actually sees)
//
// The count chain is enforced here, not assumed:
//
//   Rust PROVIDER_DEFINITIONS   the families the app actually queries
//   frontend mirror             Rust + TS_ONLY_WHITELIST (spend-only cards)
//   site copy                   frontend mirror
//
// `parse-tests/src/catalog_consistency.rs` owns the whitelist, so this script
// reads it instead of hardcoding "mcode": a new spend-only card cannot
// silently desync the website, and a whitelist entry without a mirror row
// fails here rather than in CI's Rust job.
//
//   node site/scripts/sync-site-version.mjs          # write facts + copy
//   node site/scripts/sync-site-version.mjs --check  # CI gate: fail on drift
import { readFile, writeFile } from "node:fs/promises";

const root = new URL("../../", import.meta.url);
const check = process.argv.includes("--check");

const pkg = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
const version = String(pkg.version ?? "").trim();
if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`package.json version is not a release version: ${JSON.stringify(version)}`);
}

// ── provider count chain ────────────────────────────────────────────────────
const catalogRs = await readFile(new URL("src-tauri/src/provider_catalog.rs", root), "utf8");
const rsStart = catalogRs.indexOf("const PROVIDER_DEFINITIONS");
const rsEnd = catalogRs.indexOf("\n];", rsStart);
if (rsStart < 0 || rsEnd < rsStart) {
  throw new Error("PROVIDER_DEFINITIONS not found in src-tauri/src/provider_catalog.rs");
}
const quotaFamilies = [...catalogRs.slice(rsStart, rsEnd).matchAll(/^\s*family_id:\s*"([^"]+)"/gm)].map((m) => m[1]);

const mirror = await readFile(new URL("src/providerCatalog.ts", root), "utf8");
const mirrorFamilies = [...mirror.matchAll(/familyId:\s*"([^"]+)"/g)].map((m) => m[1]);

const consistency = await readFile(new URL("parse-tests/src/catalog_consistency.rs", root), "utf8");
const whitelistMatch = consistency.match(/TS_ONLY_WHITELIST[^=]*=\s*&\[([^\]]*)\]/);
if (!whitelistMatch) throw new Error("TS_ONLY_WHITELIST not found in catalog_consistency.rs");
const spendOnly = [...whitelistMatch[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);

const missingInMirror = spendOnly.filter((id) => !mirrorFamilies.includes(id));
if (missingInMirror.length) {
  throw new Error(`whitelisted spend-only families absent from the mirror: ${missingInMirror.join(", ")}`);
}
const unexplained = mirrorFamilies.filter((id) => !quotaFamilies.includes(id) && !spendOnly.includes(id));
if (unexplained.length) {
  throw new Error(`frontend mirror has families the Rust catalog and the whitelist do not explain: ${unexplained.join(", ")}`);
}
const providerCount = mirrorFamilies.length;

// ── rewrite targets ─────────────────────────────────────────────────────────
const targets = [
  [
    "site/src/content/facts.ts",
    (src) =>
      src
        .replace(/version: "[^"]*",/, `version: "${version}",`)
        .replace(/providers: \d+,/, `providers: ${providerCount},`),
    [`version: "${version}"`, `providers: ${providerCount}`],
  ],
  [
    "site/src/i18n/index.ts",
    (src) =>
      src
        .replace(/目前收录 \d+ 个服务商/g, `目前收录 ${providerCount} 个服务商`)
        .replace(/\b\d+ providers today\b/g, `${providerCount} providers today`)
        .replace(/覆盖 \d+ 个服务商/g, `覆盖 ${providerCount} 个服务商`)
        .replace(/\b\d+ providers\b(?=[^片]*?today)/g, `${providerCount} providers`),
    [`目前收录 ${providerCount} 个服务商`, `${providerCount} providers today`],
  ],
];

const drift = [];
const rewritten = new Map();
for (const [rel, rewrite] of targets) {
  const url = new URL(rel, root);
  const before = await readFile(url, "utf8");
  const after = rewrite(before);
  rewritten.set(rel, after);
  if (before !== after) drift.push(rel);
  if (!check) await writeFile(url, after);
}

// Guard: every rewrite must have found its target, or a copy edit silently
// turned this gate into a no-op.
for (const [rel, , needles] of targets) {
  for (const needle of needles) {
    if (!rewritten.get(rel).includes(needle)) {
      throw new Error(`sync target missing in ${rel}: ${needle}`);
    }
  }
}

if (check && drift.length) {
  console.error(`site version/count drift: ${drift.join(", ")}`);
  console.error("fix with: node site/scripts/sync-site-version.mjs");
  process.exit(1);
}
console.log(`site version/count ${check ? "in sync" : "synced"}: v${version}, ${providerCount} providers (${quotaFamilies.length} quota + ${spendOnly.join("/") || "none"})`);
