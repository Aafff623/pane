interface TextMetric {
  kind: string;
  label: string;
  value?: string | null;
  detail?: string | null;
  resets_at?: number | null;
}

// Firecrawl's first row is a team-summary header (or a duplicate for one
// key). Keep the per-key balances separate: their teams may be shared.
export function overviewTextMetrics<T extends TextMetric>(metrics: readonly T[], family: string): T[] {
  const text = metrics.filter((m) => m.kind === "text" && (m.value ?? m.detail));
  if (family === "firecrawl" && text.length > 1) return text.slice(1);
  return text.slice(0, 1);
}

interface TextQuota {
  valueText?: string | null;
}

// Text-only providers (balances, notes) have no ratio to draw, but their
// reading is more useful than a question mark. Keep it short enough for the
// percent/ring slot; longer copy stays in the foot line.
export function overviewTextHeadline(quota: TextQuota, maxChars: number): string | null {
  const value = quota.valueText?.trim();
  if (!value || value.length > maxChars) return null;
  return value;
}
