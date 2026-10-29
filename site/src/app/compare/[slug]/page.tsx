import Link from "next/link";
import { notFound } from "next/navigation";

import { Check } from "lucide-react";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import { JsonLd, breadcrumbSchema } from "@/components/site/json-ld";
import { RelatedLinks } from "@/components/site/related-links";
import { Button } from "@/components/ui/button";
import {
  comparisons,
  getComparison,
  isComparisonIndexable,
} from "@/content/comparisons";
import { withNavRef } from "@/lib/nav-ref";

export function generateStaticParams() {
  return comparisons.map((c) => ({ slug: c.slug }));
}

export const dynamicParams = false;

const comparisonTitles: Record<string, string> = {
  "design-mode-vs-cursor-design-mode": "Design Mode vs Cursor visual editor",
  "design-mode-vs-builder-io-visual-copilot":
    "Design Mode vs Builder.io Visual Copilot",
};

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const c = getComparison(slug);
  if (!c) return {};
  const url = `https://designmode.app/compare/${c.slug}`;
  const title = comparisonTitles[c.slug] ?? c.metaTitle;
  const description = c.metaDescription;
  return {
    title: { absolute: title },
    description,
    keywords: c.keywords,
    robots: { index: isComparisonIndexable(c), follow: true },
    alternates: { canonical: url },
    openGraph: {
      title,
      description,
      url,
      type: "website",
      images: [
        {
          url: "/og-image.png",
          width: 1200,
          height: 630,
          alt: "Design Mode — The visual editor for all your agent’s work",
        },
      ],
    },
    twitter: {
      card: "summary_large_image",
      title,
      description,
      images: [
        {
          url: "/og-image.png",
          width: 1200,
          height: 630,
          alt: "Design Mode — The visual editor for all your agent’s work",
        },
      ],
    },
  };
}

export default async function ComparePage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const c = getComparison(slug);
  if (!c) notFound();

  const url = `https://designmode.app/compare/${c.slug}`;

  return (
    <>
      <JsonLd
        data={breadcrumbSchema([
          { name: "Home", url: "https://designmode.app/" },
          { name: "Comparisons", url: "https://designmode.app/compare" },
          { name: c.title, url },
        ])}
      />

      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-4xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              {c.title}
            </h1>
            <p className="text-muted-foreground mt-4 text-base">
              {c.oneLiner}
            </p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-4xl" />
        <article className="container mt-12 max-w-4xl space-y-12">
          <aside className="bg-muted text-muted-foreground rounded-xl border p-4 text-base leading-relaxed">
            <strong className="text-foreground">Research status:</strong>{" "}
            {isComparisonIndexable(c)
              ? "Source-backed comparison. Review the dated sources and methodology below for what was documented, inspected or tested. Pricing and capabilities can change."
              : "Unverified research draft, excluded from search indexing. Competitor claims need first-party sources and a dated methodology before publication."}
          </aside>
          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">
              How they compare
            </h2>
            <p className="text-muted-foreground mt-4 leading-relaxed">
              {c.positioning}
            </p>
          </div>

          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">
              Feature-by-feature
            </h2>
            <div className="mt-6 overflow-x-auto">
              <table className="w-full border-collapse text-base">
                <caption className="sr-only">
                  Feature comparison between Design Mode and {c.competitor}
                </caption>
                <thead>
                  <tr className="border-b">
                    <th
                      scope="col"
                      className="text-muted-foreground w-1/3 py-4 pr-4 text-left font-semibold"
                    >
                      Feature
                    </th>
                    <th
                      scope="col"
                      className="text-foreground py-4 pr-4 text-left font-semibold"
                    >
                      Design Mode
                    </th>
                    <th
                      scope="col"
                      className="text-foreground py-4 text-left font-semibold"
                    >
                      {c.competitor}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {c.table.map((row, i) => (
                    <tr key={i} className="border-b align-top">
                      <th
                        scope="row"
                        className="text-muted-foreground py-4 pr-4 text-left font-medium"
                      >
                        {row.feature}
                      </th>
                      <td className="text-foreground py-4 pr-4 leading-relaxed">
                        {row.designMode}
                      </td>
                      <td className="text-foreground py-4 leading-relaxed">
                        {row.competitor}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="grid gap-8 md:grid-cols-2">
            <div>
              <h2 className="text-2xl tracking-tight md:text-3xl">
                When to pick Design Mode
              </h2>
              <ul className="mt-4 space-y-2">
                {c.whenToPickDesignMode.map((item, i) => (
                  <li
                    key={i}
                    className="text-muted-foreground flex gap-2 text-base leading-relaxed"
                  >
                    <Check className="text-foreground mt-2 size-4 shrink-0" />
                    <span>{item}</span>
                  </li>
                ))}
              </ul>
            </div>
            <div>
              <h2 className="text-2xl tracking-tight md:text-3xl">
                When to pick {c.competitor}
              </h2>
              <ul className="mt-4 space-y-2">
                {c.whenToPickCompetitor.map((item, i) => (
                  <li
                    key={i}
                    className="text-muted-foreground flex gap-2 text-base leading-relaxed"
                  >
                    <Check className="text-foreground mt-2 size-4 shrink-0" />
                    <span>{item}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>

          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">Honest take</h2>
            <p className="text-muted-foreground mt-4 leading-relaxed">
              {c.honesty}
            </p>
          </div>

          {c.research && (
            <section aria-label="Comparison sources">
              <h2 className="text-2xl tracking-tight md:text-3xl">
                Sources and scope
              </h2>
              <p className="text-muted-foreground mt-4 text-base leading-relaxed">
                Sources checked {c.research.checkedOn}. {c.research.methodology}
              </p>
              <ul className="mt-4 space-y-2 text-base">
                {c.research.sources.map((source) => (
                  <li key={source.url}>
                    <a
                      href={withNavRef(source.url)}
                      className="underline underline-offset-8"
                    >
                      {source.label}
                    </a>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <div className="flex flex-wrap items-center gap-4 pt-4">
            <Button asChild>
              <Link href="/demo">Try Design Mode</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/use-cases">See use cases</Link>
            </Button>
          </div>
        </article>

        <RelatedLinks
          title="More comparisons"
          links={c.related
            .map((slug) => getComparison(slug))
            .filter((r): r is NonNullable<typeof r> => Boolean(r))
            .map((r) => ({
              href: `/compare/${r.slug}`,
              title: r.title,
              description: r.oneLiner,
            }))}
        />
      </section>
    </>
  );
}
