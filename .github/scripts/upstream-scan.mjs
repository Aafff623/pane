#!/usr/bin/env node
// Upstream reference-repo scanner (see docs/reference-repos.md).
//
// Reads the machine-readable baseline embedded in docs/reference-repos.md,
// queries the public GitHub API for each repository (latest release + default
// branch HEAD), prints a markdown report, and writes it to
// `upstream-scan-report.md` (and $GITHUB_STEP_SUMMARY when present).
//
// Modes:
//   node .github/scripts/upstream-scan.mjs                 # report only
//   node .github/scripts/upstream-scan.mjs --no-fail-on-drift
//   node .github/scripts/upstream-scan.mjs --update        # rewrite baseline + table
//
// Exit codes: 0 ok · 1 release drift (unless --no-fail-on-drift) · 2 scan error.
// No repository secret is required: GITHUB_TOKEN / GH_TOKEN only raises the API
// rate limit and is optional (the public endpoints are used when absent).

import { readFileSync, writeFileSync, appendFileSync } from "node:fs";

const DOC = "docs/reference-repos.md";
const REPORT = "upstream-scan-report.md";

const BEGIN = "<!-- upstream-scan:begin -->";
const END = "<!-- upstream-scan:end -->";
const TBEGIN = "<!-- upstream-scan-table:begin -->";
const TEND = "<!-- upstream-scan-table:end -->";

const TABLE_HEADER = "| repository | ref | 复核 release | 线上 release | 默认分支 HEAD | status |";
const TABLE_SEP = "| --- | --- | --- | --- | --- | --- |";

const argv = new Set(process.argv.slice(2));
const UPDATE = argv.has("--update");
const FAIL_ON_DRIFT = !argv.has("--no-fail-on-drift");

const TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "";
const API = "https://api.github.com";
const HEADERS = { Accept: "application/vnd.github+json", "User-Agent": "pane-upstream-scan" };
if (TOKEN) HEADERS.Authorization = `Bearer ${TOKEN}`;

function die(msg, code = 2) {
  console.error(`upstream-scan: ${msg}`);
  process.exit(code);
}

function region(text, begin, end) {
  const i = text.indexOf(begin);
  const j = text.indexOf(end);
  if (i < 0 || j < 0 || j <= i) die(`markers not found: ${begin} / ${end}`);
  return { start: i, stop: j + end.length, body: text.slice(i + begin.length, j) };
}

function parseBaseline(text) {
  const { body } = region(text, BEGIN, END);
  const m = body.match(/```json\s*([\s\S]*?)```/);
  if (!m) die("no ```json block inside the upstream-scan region");
  let obj;
  try {
    obj = JSON.parse(m[1]);
  } catch (e) {
    die(`baseline JSON parse failed: ${e.message}`);
  }
  if (!Array.isArray(obj.repos)) die("baseline.repos must be an array");
  return obj;
}

async function api(path) {
  const res = await fetch(`${API}${path}`, { headers: HEADERS });
  if (res.status === 404) return null;
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`GET ${path} -> ${res.status} ${res.statusText}${body ? `: ${body.slice(0, 160)}` : ""}`);
  }
  return res.json();
}

async function scan(entry) {
  const repo = entry.repo;
  const meta = await api(`/repos/${repo}`);
  if (!meta) throw new Error(`repository ${repo} not found`);
  const ref = entry.ref || meta.default_branch || "main";
  const commit = await api(`/repos/${repo}/commits/${encodeURIComponent(ref)}`);
  if (!commit) throw new Error(`no commit for ${repo}@${ref}`);
  const rel = await api(`/repos/${repo}/releases/latest`);
  return {
    repo,
    ref,
    defaultBranch: meta.default_branch || ref,
    sha: commit.sha,
    shaShort: String(commit.sha || "").slice(0, 8),
    commitDate: commit.commit?.committer?.date || commit.commit?.author?.date || null,
    commitMsg: String(commit.commit?.message || "").split("\n")[0],
    release: rel?.tag_name || null,
    releaseAt: rel?.published_at || null,
  };
}

function statusOf(base, live) {
  if (base.release && live.release && base.release !== live.release) return "release-moved";
  if (base.release && !live.release) return "release-missing";
  if (base.sha && live.sha && base.sha !== live.sha) return "commit-moved";
  return "up-to-date";
}

function tableRow(base, live, status) {
  const head = live.shaShort
    ? `${live.shaShort} (${(live.commitDate || "").slice(0, 10)})`
    : "—";
  return `| ${live.repo} | ${live.ref} | ${base.release || "—"} | ${live.release || "—"} | ${head} | ${status} |`;
}

function buildTable(rows) {
  return [TABLE_HEADER, TABLE_SEP, ...rows].join("\n");
}

function baselineJson(baseline) {
  const lines = baseline.repos.map((r) => {
    const parts = [`"repo": ${JSON.stringify(r.repo)}`];
    if (r.ref) parts.push(`"ref": ${JSON.stringify(r.ref)}`);
    if (r.sha) parts.push(`"sha": ${JSON.stringify(r.sha)}`);
    if (r.release) parts.push(`"release": ${JSON.stringify(r.release)}`);
    return `    { ${parts.join(", ")} }`;
  });
  return `{\n  "checkedAt": ${JSON.stringify(baseline.checkedAt)},\n  "repos": [\n${lines.join(",\n")}\n  ]\n}`;
}

function replaceRegion(text, begin, end, newBody) {
  const { start, stop } = region(text, begin, end);
  return text.slice(0, start) + begin + newBody + end + text.slice(stop);
}

function todayUtc() {
  return new Date().toISOString().slice(0, 10);
}

async function main() {
  const doc = readFileSync(DOC, "utf8");
  const baseline = parseBaseline(doc);
  const byRepo = new Map(baseline.repos.map((r) => [r.repo, r]));

  const results = [];
  const errors = [];
  for (const entry of baseline.repos) {
    try {
      results.push(await scan(entry));
    } catch (e) {
      errors.push({ repo: entry.repo, message: e.message });
    }
  }

  const rows = [];
  const drift = [];
  const commitMoved = [];
  for (const live of results) {
    const base = byRepo.get(live.repo) || {};
    const status = statusOf(base, live);
    rows.push(tableRow(base, live, status));
    if (status === "release-moved" || status === "release-missing") drift.push({ live, base, status });
    else if (status === "commit-moved") commitMoved.push(live);
  }

  const generatedAt = new Date().toISOString();
  const lines = [];
  lines.push("# Upstream reference scan");
  lines.push("");
  lines.push(`- Generated: ${generatedAt}`);
  lines.push(`- Mode: ${UPDATE ? "update" : "report"}`);
  lines.push(`- Repos checked: ${results.length}/${baseline.repos.length}`);
  lines.push(`- Release drift: ${drift.length} · commit drift: ${commitMoved.length} · errors: ${errors.length}`);
  lines.push("");
  lines.push("## Results");
  lines.push("");
  lines.push(buildTable(rows));
  lines.push("");
  if (drift.length) {
    lines.push("## Actionable (new upstream release)");
    lines.push("");
    for (const { live, base, status } of drift) {
      lines.push(`- **${live.repo}** — ${status}: baseline \`${base.release || "—"}\` → live \`${live.release || "—"}\` · https://github.com/${live.repo}/releases`);
    }
    lines.push("");
  }
  if (commitMoved.length) {
    lines.push("## Informational (default-branch moved, no new release)");
    lines.push("");
    for (const live of commitMoved) {
      lines.push(`- **${live.repo}** — HEAD \`${live.shaShort}\` (${(live.commitDate || "").slice(0, 10)}): ${live.commitMsg}`);
    }
    lines.push("");
  }
  if (errors.length) {
    lines.push("## Scan errors");
    lines.push("");
    for (const e of errors) lines.push(`- **${e.repo}** — ${e.message}`);
    lines.push("");
  }
  lines.push("## Details");
  lines.push("");
  for (const live of results) {
    lines.push(`- **${live.repo}** — ref \`${live.ref}\` · release \`${live.release || "—"}\`${live.releaseAt ? ` (${live.releaseAt.slice(0, 10)})` : ""} · HEAD \`${live.shaShort}\` (${(live.commitDate || "").slice(0, 10)}): ${live.commitMsg}`);
  }
  lines.push("");

  const report = lines.join("\n");
  writeFileSync(REPORT, report, "utf8");
  console.log(report);
  if (process.env.GITHUB_STEP_SUMMARY) {
    try {
      appendFileSync(process.env.GITHUB_STEP_SUMMARY, report + "\n", "utf8");
    } catch {
      /* summary is best-effort */
    }
  }

  if (UPDATE) {
    if (errors.length) die("refusing to update the baseline while scan errors remain", 2);
    const next = {
      checkedAt: todayUtc(),
      repos: results.map((live) => ({
        repo: live.repo,
        ref: live.ref,
        sha: live.sha,
        release: live.release,
      })),
    };
    let out = doc;
    out = replaceRegion(out, BEGIN, END, `\n\`\`\`json\n${baselineJson(next)}\n\`\`\`\n`);
    out = replaceRegion(out, TBEGIN, TEND, `\n${buildTable(rows)}\n`);
    writeFileSync(DOC, out, "utf8");
    console.log(`upstream-scan: baseline updated in ${DOC}`);
    process.exit(0);
  }

  if (errors.length) die(`${errors.length} repo(s) could not be checked`, 2);
  if (FAIL_ON_DRIFT && drift.length) die(`${drift.length} repo(s) published a new release past the baseline`, 1);
  process.exit(0);
}

main().catch((e) => die(e.stack || String(e), 2));
