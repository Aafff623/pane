import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// The usage-stats charts (daily model trend + model donut) were rebuilt to
// match the card trend the owner praised (2026-11-26). These locks keep the
// two traps that made the old versions look broken:
//
//   1. `stop-color: var(--x)` computes to rgb(0,0,0) in Chromium, so the area
//      under the curve filled BLACK. The colour must arrive via `currentColor`
//      from a group that owns the CSS variable.
//   2. A polyline reports "points exist", not a shape. The series must be a
//      smoothed path with a gradient area beneath it.
//
// Exercised against the REAL functions lifted out of main.ts, so the markup
// asserted here is the markup that ships.

const mainSrc = readFileSync(new URL("../src/main.ts", import.meta.url), "utf8");
const main = ts.createSourceFile("main.ts", mainSrc, ts.ScriptTarget.ES2020, true);

const WANTED = new Set([
  "statsSmoothPath",
  "statsLineChartHtml",
  "statsDonutHtml",
  "statsEmptyHtml",
]);
const extracted = main.statements
  .filter((node) => ts.isFunctionDeclaration(node) && WANTED.has(node.name?.text))
  .map((node) => node.getText(main))
  .join("\n");

function harness() {
  const code = `
    const STATS_CHART_WIDTH = 600, STATS_CHART_HEIGHT = 150;
    ${extracted}
    const dict = {
      "stats.total": "total", "stats.noUsage": "none", "stats.tokenUnit": "tokens",
      "stats.other": "Other", "stats.noTrend": "no trend", "stats.noTrendHint": "hint",
      "stats.noModels": "no models", "stats.noModelsHint": "hint",
      "stats.trendTitle": "trend", "stats.donutTitle": "donut",
    };
    const t = (k, v) => { let s = dict[k] ?? k; for (const [n, x] of Object.entries(v || {})) s = s.split("{" + n + "}").join(x); return s; };
    const fmtTokens = (n) => n >= 1e6 ? (n / 1e6).toFixed(1) + "M" : n >= 1e3 ? (n / 1e3).toFixed(1) + "K" : String(Math.round(n));
    const escapeHtml = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
    const statsPercent = (v) => v.toFixed(1) + "%";
    const statsDayLabel = (d) => d.slice(5);
    const statsAxisIndexes = (n) => n <= 14 ? [...Array(n).keys()] : [0, 5, 10, 15, 20, 25, 29].filter((i) => i < n);
    const statsChartPeak = (days, perDay, top) => Math.max(...days.flatMap((d) => top.map((m) => perDay.get(d)?.get(m.model) ?? 0)), 1);
    function statsEmptyHtml(a) { return '<div class="stats-empty">' + a + '</div>'; }
    function foldDonutSlices(models, otherLabel) {
      const total = models.reduce((s, m) => s + m.tokens, 0) || 1;
      const sorted = [...models].sort((a, b) => b.tokens - a.tokens);
      const top = sorted.slice(0, 5);
      const rest = sorted.slice(5);
      const out = top.map((m) => ({ label: m.model, tokens: m.tokens, share: m.tokens / total }));
      if (rest.length) {
        const sum = rest.reduce((s, m) => s + m.tokens, 0);
        out.push({ label: otherLabel, tokens: sum, share: sum / total });
      }
      return out;
    }
  `;
  const context = vm.createContext({});
  vm.runInContext(ts.transpile(code, { target: ts.ScriptTarget.ES2020 }), context);
  return context;
}

function sampleData(gapDays = []) {
  const days = Array.from({ length: 30 }, (_, i) => `2026-11-${String(i + 1).padStart(2, "0")}`);
  const dayModels = new Map();
  const models = [
    { model: "claude", tokens: 3000 },
    { model: "codex", tokens: 2000 },
    { model: "gemini", tokens: 1000 },
  ];
  models.forEach((model) => {
    days.forEach((day, index) => {
      // A gap is a day with NO row for that model: not a measured zero.
      if (gapDays.includes(index)) return;
      if (!dayModels.has(day)) dayModels.set(day, new Map());
      dayModels.get(day).set(model.model, model.tokens / 30 + index);
    });
  });
  return { days, dayModels, models };
}

test("trend series are smoothed cubic paths, not polylines", () => {
  const ctx = harness();
  const { days, dayModels, models } = sampleData();
  const html = ctx.statsLineChartHtml(days, dayModels, models);
  assert.doesNotMatch(html, /<polyline/, "a polyline shows points, not a shape");
  assert.match(html, /class="stats-line" d="M[^"]*C/, "the line must be a cubic path through the data");
});

test("each series carries a gradient area beneath it", () => {
  const ctx = harness();
  const { days, dayModels, models } = sampleData();
  const html = ctx.statsLineChartHtml(days, dayModels, models);
  const areas = html.match(/class="stats-line-area"/g) ?? [];
  assert.ok(areas.length >= 1, "the area under the curve is what makes the shape readable");
  assert.match(html, /fill="url\(#stats-grad-0\)"/);
});

test("gradient stops never carry a CSS var directly", () => {
  // Chromium computes `stop-color: var(--x)` to rgb(0,0,0) — the area filled
  // black. The colour must arrive through currentColor from a styled parent.
  const ctx = harness();
  const { days, dayModels, models } = sampleData();
  const html = ctx.statsLineChartHtml(days, dayModels, models);
  const stopTags = html.match(/<stop[^>]*>/g) ?? [];
  assert.ok(stopTags.length > 0, "the gradient needs stops");
  for (const tag of stopTags) {
    assert.match(tag, /stop-color="currentColor"/, "stops must use currentColor");
    assert.doesNotMatch(tag, /stop-color="var\(/, "a CSS var here computes to black");
  }
  assert.match(html, /<linearGradient[^>]*style="color:var\(--stats-c1\)"/, "the gradient owns its colour");
});

test("a day with no row breaks the line instead of drawing zero", () => {
  const ctx = harness();
  // Gap on day 10: the path must break into separate runs there.
  const { days, dayModels, models } = sampleData([10]);
  const html = ctx.statsLineChartHtml(days, dayModels, models);
  const paths = html.match(/class="stats-line" d="[^"]*"/g) ?? [];
  assert.ok(paths.length > models.length, "a gap splits one series into two paths");
  // A single measured day must render as a dot, not a degenerate path.
  const sparse = sampleData(Array.from({ length: 29 }, (_, i) => i));
  const sparseHtml = ctx.statsLineChartHtml(sparse.days, sparse.dayModels, sparse.models);
  assert.match(sparseHtml, /stats-line-dot is-only/, "one measured day is a dot");
});

test("the donut leaves a real gap between slices and labels each share", () => {
  const ctx = harness();
  const { models } = sampleData();
  const totals = models.map((model) => ({ model: model.model, tokens: model.tokens, cost: 0 }));
  const html = ctx.statsDonutHtml(totals);
  const slices = html.match(/stroke-dasharray="[\d.]+ ([\d.]+)"/g) ?? [];
  assert.ok(slices.length >= 2, "each slice is its own arc");
  // The old 0.8-unit gap on a 23px band read as one continuous ring.
  const gaps = slices.map((s) => Number(s.match(/ ([\d.]+)"/)[1]));
  assert.ok(gaps.every((gap) => gap >= 3), `slices must be visibly separated, got ${gaps}`);
  assert.match(html, /class="stats-donut-bar"/, "every row carries a share bar");
  assert.match(html, /class="stats-donut-total"/, "the ring keeps its centre total");
});
