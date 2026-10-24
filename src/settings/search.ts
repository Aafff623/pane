/// Cross-page settings search: a pure index shape plus matching/selection
/// helpers. The index itself is collected from the rendered rows (every row
/// carries `data-setting-anchor`, including rows adopted from the legacy
/// sheet), so a new setting cannot be searchable-but-unrendered or rendered
/// but missing from a hand-maintained list.
export interface SettingsSearchEntry {
  anchor: string;
  label: string;
  section: string;
  page: string;
}

export interface SearchCoverage {
  /// Rendered anchors the index does not know about.
  missing: string[];
  /// Index entries whose row is no longer rendered.
  orphaned: string[];
}

export function normalizeSearchQuery(value: string): string {
  return value.trim().toLowerCase();
}

/// Keep one result per anchor; label and section both match, so searching a
/// page name ("proxy") still finds rows whose label does not repeat it.
export function filterSearchEntries(
  entries: readonly SettingsSearchEntry[],
  query: string,
  limit = 12,
): SettingsSearchEntry[] {
  const needle = normalizeSearchQuery(query);
  if (!needle) return [];
  const seen = new Set<string>();
  const hits: SettingsSearchEntry[] = [];
  for (const entry of entries) {
    if (seen.has(entry.anchor)) continue;
    const haystack = `${entry.label}\n${entry.section}`.toLowerCase();
    if (!haystack.includes(needle)) continue;
    seen.add(entry.anchor);
    hits.push(entry);
    if (hits.length >= limit) break;
  }
  return hits;
}

/// Arrow navigation wraps in both directions.
export function stepSelection(selected: number, length: number, delta: number): number {
  if (length <= 0) return -1;
  return (selected + delta + length) % length;
}

/// The two-way check behind the acceptance test: no rendered row is silently
/// missing, and no index entry points at a row that is gone.
export function searchCoverage(
  entries: readonly SettingsSearchEntry[],
  renderedAnchors: readonly string[],
): SearchCoverage {
  const inIndex = new Set(entries.map((entry) => entry.anchor));
  const rendered = new Set(renderedAnchors);
  return {
    missing: [...rendered].filter((anchor) => !inIndex.has(anchor)),
    orphaned: [...inIndex].filter((anchor) => !rendered.has(anchor)),
  };
}
