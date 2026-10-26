import test from "node:test";
import assert from "node:assert/strict";
import {
  aggregateModelDayTokens,
  aggregateModelTotals,
  foldDonutSlices,
  statsAxisIndexes,
  statsChartPeak,
} from "../src/statsMath.ts";

const rows = [
  { day: "2026-10-01", tokens: 300, cost: 0.3, models: [{ model: "a", tokens: 200, cost: 0.2 }, { model: "b", tokens: 100, cost: 0.1 }] },
  { day: "2026-10-02", tokens: 500, cost: 0.5, models: [{ model: "a", tokens: 500, cost: 0.5 }] },
  { day: "2026-10-03", tokens: 90, cost: 0.09, models: [] },
];

test("sparse X labels: all days up to 14, 5-day stride to 45, weekly beyond", () => {
  assert.deepEqual(statsAxisIndexes(7), [0, 1, 2, 3, 4, 5, 6]);
  assert.deepEqual(statsAxisIndexes(14), Array.from({ length: 14 }, (_, i) => i));
  assert.deepEqual(statsAxisIndexes(30), [0, 5, 10, 15, 20, 25, 29]);
  assert.deepEqual(statsAxisIndexes(60), [0, 7, 14, 21, 28, 35, 42, 49, 56, 59]);
  assert.deepEqual(statsAxisIndexes(0), []);
});

test("model aggregation merges providers by model and keeps unknown rows", () => {
  const totals = aggregateModelTotals(rows, "unknown");
  assert.deepEqual(totals.map((model) => [model.model, model.tokens]), [
    ["a", 700],
    ["b", 100],
    ["unknown", 90],
  ]);
  const byDay = aggregateModelDayTokens(rows, "unknown");
  assert.equal(byDay.get("2026-10-01").get("a"), 200);
  assert.equal(byDay.get("2026-10-03").get("unknown"), 90);
});

test("Y peak follows a single visible point, not the day sum", () => {
  const dayModels = aggregateModelDayTokens(rows, "unknown");
  const totals = aggregateModelTotals(rows, "unknown");
  assert.equal(statsChartPeak(["2026-10-01", "2026-10-02"], dayModels, totals), 500);
  assert.equal(statsChartPeak([], dayModels, totals), 0);
});

test("donut folds more than six positive models into one other slice", () => {
  const models = Array.from({ length: 8 }, (_, index) => ({
    model: `m${index}`,
    tokens: 100 - index * 10,
    cost: 0,
  }));
  const slices = foldDonutSlices(models, "other");
  assert.equal(slices.length, 6);
  assert.equal(slices.at(-1).label, "other");
  assert.ok(Math.abs(slices.reduce((sum, slice) => sum + slice.share, 0) - 1) < 1e-9);
  assert.deepEqual(foldDonutSlices([], "other"), []);
  assert.equal(foldDonutSlices(models.slice(0, 6), "other").length, 6);
});
