/// Estimated Step Plan Credits from local token usage.
///
/// Step Plan bills in Credits at `1M Credits = ¥1`. The official CNY token
/// prices below are per 1M tokens, so `Credits = tokens × price` (the 1M
/// cancels). Local usage rows only carry a total token count per model, not
/// the input/output/cache split, so this assumes 70% input / 30% output and
/// charges all input at the cache-miss rate. Peak hours and cache hits can
/// change the real bill (Step 5 Preview is 2× in peak hours), hence the
/// caller must always label the result as an estimate.
export interface StepfunTokenRow {
  model: string;
  tokens: number;
}

export interface StepfunCreditEstimate {
  credits: number;
  unknownTokens: number;
  unknownModels: string[];
}

// Official StepFun CNY prices per 1M tokens (2026-10).
const RATES: ReadonlyArray<{ match: RegExp; input: number; output: number }> = [
  { match: /^step-5-preview/, input: 7, output: 20 },
  { match: /^step-3\.7-flash/, input: 1.35, output: 8.1 },
  { match: /^step-3\.5-flash/, input: 0.7, output: 2.1 },
];

const INPUT_SHARE = 0.7;

function modelKey(raw: string): string {
  const last = raw.toLowerCase().split(/[·/:]/).pop() ?? "";
  return last.trim();
}

export function estimateStepfunCredits(rows: readonly StepfunTokenRow[]): StepfunCreditEstimate {
  let credits = 0;
  let unknownTokens = 0;
  const unknownModels: string[] = [];
  for (const row of rows) {
    if (!Number.isFinite(row.tokens) || row.tokens <= 0) continue;
    const rate = RATES.find((entry) => entry.match.test(modelKey(row.model)));
    if (!rate) {
      unknownTokens += row.tokens;
      unknownModels.push(row.model);
      continue;
    }
    const blended = INPUT_SHARE * rate.input + (1 - INPUT_SHARE) * rate.output;
    credits += row.tokens * blended;
  }
  return {
    credits,
    unknownTokens,
    unknownModels: [...new Set(unknownModels)],
  };
}
