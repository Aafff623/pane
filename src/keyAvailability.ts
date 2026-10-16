// Search / MCP providers (Bocha, Tavily, Firecrawl, Brave) keep every api-key
// as one metric row on a SINGLE snapshot instead of one snapshot per account,
// so the multi-account badge path never sees them. Their key rows carry the
// backend's per-round availability flag; headline and summary rows leave it
// null. Counting that flag keeps "available" honest — the row text carries
// both real readings and failure messages.
export interface KeyAvailabilityMetric {
  available?: boolean | null;
}

/// Every flagged key row of one snapshot, in metric order — the rows the
/// overview tile expands to and the rows the badge counts. Headline and
/// summary rows carry no flag, so a provider that is not a key card yields
/// an empty list.
export function keyAvailabilityRows<T extends KeyAvailabilityMetric>(
  metrics: readonly T[] | null | undefined,
): T[] {
  return (metrics ?? []).filter(
    (m) => m.available === true || m.available === false,
  );
}

/// Every flagged key row, counted — including the 0- and 1-row cases the
/// badge summary folds away. The card fold needs the exact count: 2+ keys
/// fold like a multi-account card, a lone key offers no fold control at all,
/// and 0 flagged rows mean the snapshot is not a key card.
export function keyAvailabilityCounts<T extends KeyAvailabilityMetric>(
  metrics: readonly T[] | null | undefined,
): { total: number; available: number } {
  const rows = keyAvailabilityRows(metrics);
  return { total: rows.length, available: rows.filter((m) => m.available === true).length };
}

/// How many rows an overview tile can fold. Multi-account families fold
/// their account snapshots; the search/MCP families fold their api-keys
/// instead, because every key is a metric row on the family's single
/// snapshot. Both paths fold only at 2+ rows, so a lone account or a lone
/// key renders no control — and the two counts never mix: a family is one
/// or the other, never both.
export function overviewFoldCount(accountCount: number, keyTotal: number): number {
  return Math.max(accountCount, keyTotal);
}

/// Available/total api-key counts for one snapshot's metric rows, or null
/// when it holds fewer than two key rows: a single key needs no badge, and a
/// snapshot from any other provider has no flagged rows at all.
export function keyAvailabilitySummary<T extends KeyAvailabilityMetric>(
  metrics: readonly T[] | null | undefined,
): { total: number; available: number } | null {
  const counts = keyAvailabilityCounts(metrics);
  return counts.total <= 1 ? null : counts;
}
