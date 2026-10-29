import { comparisons, isComparisonIndexable } from "@/content/comparisons";

export const dynamic = "force-static";

export function GET() {
  const entries = comparisons
    .filter(isComparisonIndexable)
    .map((comparison) => {
      const research = comparison.research!;
      return [
        `## ${comparison.title}`,
        `URL: https://designmode.app/compare/${comparison.slug}`,
        `Sources checked: ${research.checkedOn}`,
        comparison.oneLiner,
        comparison.positioning,
        ...comparison.table.map(
          (row) =>
            `- ${row.feature}: Design Mode — ${row.designMode} ${comparison.competitor} — ${row.competitor}`,
        ),
        `Choose Design Mode: ${comparison.whenToPickDesignMode.join(" ")}`,
        `Choose ${comparison.competitor}: ${comparison.whenToPickCompetitor.join(" ")}`,
        comparison.honesty,
        `Methodology: ${research.methodology}`,
        ...research.sources.map(
          (source) => `Source: ${source.label} — ${source.url}`,
        ),
      ].join("\n\n");
    });

  return new Response(
    [
      "# Design Mode comparisons",
      "Publisher: Design Mode. These are vendor-authored comparisons, not independent reviews. Sources and test limitations accompany each entry. Prices and capabilities can change. Unverified drafts are excluded.",
      ...entries,
    ].join("\n\n"),
    { headers: { "Content-Type": "text/plain; charset=utf-8" } },
  );
}
