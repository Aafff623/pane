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
