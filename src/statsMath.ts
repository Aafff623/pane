// Pure aggregation for the usage stats page (#64-04/05). Kept out of main.ts
// so the chart rules — Y peak, zero filtering, sparse ticks, >6 folding — can
// be tested without a DOM.

export interface StatsDailyModel {
  model: string;
  tokens: number;
  cost: number;
}

export interface StatsDailyRow {
  day: string;
  tokens: number;
  cost: number;
  models: readonly StatsDailyModel[];
}

export interface StatsModelTotal {
  model: string;
  tokens: number;
  cost: number;
}

export interface StatsDonutSlice {
  label: string;
  tokens: number;
  share: number;
}

/// ZCode's X-axis rules: every day up to two weeks, a 5-day stride up to 45
/// days, then weekly; the first and last labels always stay.
export function statsAxisIndexes(total: number): number[] {
  if (total <= 0) return [];
  if (total <= 14) return Array.from({ length: total }, (_, index) => index);
  const step = total > 45 ? 7 : 5;
  const indexes = new Set<number>([0, total - 1]);
  for (let index = 0; index < total; index += step) indexes.add(index);
  return [...indexes].sort((a, b) => a - b);
}

/// Rows without a model breakdown still count: they land under one "unknown"
/// label so the chart total matches the heatmap instead of losing spend.
export function aggregateModelDayTokens(
  rows: readonly StatsDailyRow[],
  unknownModel: string,
): Map<string, Map<string, number>> {
  const out = new Map<string, Map<string, number>>();
  for (const row of rows) {
    const day = out.get(row.day) ?? new Map<string, number>();
    if (row.models.length) {
      for (const model of row.models) {
        day.set(model.model, (day.get(model.model) ?? 0) + model.tokens);
      }
    } else if (row.tokens > 0) {
      day.set(unknownModel, (day.get(unknownModel) ?? 0) + row.tokens);
    }
    out.set(row.day, day);
  }
  return out;
}

export function aggregateModelTotals(
  rows: readonly StatsDailyRow[],
  unknownModel: string,
): StatsModelTotal[] {
  const out = new Map<string, StatsModelTotal>();
  const add = (model: string, tokens: number, cost: number) => {
    const cell = out.get(model) ?? { model, tokens: 0, cost: 0 };
    cell.tokens += tokens;
    cell.cost += cost;
    out.set(model, cell);
  };
  for (const row of rows) {
    if (row.models.length) {
      for (const model of row.models) add(model.model, model.tokens, model.cost);
    } else if (row.tokens > 0) {
      add(unknownModel, row.tokens, row.cost);
    }
  }
  return [...out.values()].sort((a, b) => b.tokens - a.tokens);
}

/// The independent-line Y range: the largest single visible point, never the
/// per-day sum across models (that would flatten every line).
export function statsChartPeak(
  days: readonly string[],
  dayModels: Map<string, Map<string, number>>,
  models: readonly StatsModelTotal[],
): number {
  let peak = 0;
  for (const model of models.slice(0, 6)) {
    for (const day of days) {
      peak = Math.max(peak, dayModels.get(day)?.get(model.model) ?? 0);
    }
  }
  return peak;
}

/// Top six models; when there are more, keep five plus one "other" slice.
export function foldDonutSlices(
  models: readonly StatsModelTotal[],
  otherLabel: string,
  maxSlices = 6,
): StatsDonutSlice[] {
  const positive = models.filter((model) => model.tokens > 0);
  const total = positive.reduce((sum, model) => sum + model.tokens, 0);
  if (total <= 0) return [];
  const merge = positive.length > maxSlices;
  const limit = merge ? maxSlices - 1 : maxSlices;
  const sizes = positive.slice(0, limit).map((model) => ({
    label: model.model,
    tokens: model.tokens,
  }));
  if (merge) {
    const restTokens = positive
      .slice(limit)
      .reduce((sum, model) => sum + model.tokens, 0);
    if (restTokens > 0) sizes.push({ label: otherLabel, tokens: restTokens });
  }
  return sizes.map((slice) => ({ ...slice, share: slice.tokens / total }));
}
