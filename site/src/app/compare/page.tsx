import Link from "next/link";

import { ArrowUpRight } from "lucide-react";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import { JsonLd, collectionPageSchema } from "@/components/site/json-ld";
import { comparisons, isComparisonIndexable } from "@/content/comparisons";

const pageTitle = "Browser visual editors and AI feedback tools";
const pageDescription =
  "Compare live-page editors, browser inspectors, annotation tools, Figma-to-code products and editor-native design modes by the workflow they actually support.";

export const metadata = {
  title: { absolute: `${pageTitle} | Design Mode` },
  description: pageDescription,
  alternates: { canonical: "https://designmode.app/compare" },
  openGraph: {
    type: "website",
    title: `${pageTitle} | Design Mode`,
    description: pageDescription,
    url: "https://designmode.app/compare",
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
    title: `${pageTitle} | Design Mode`,
    description: pageDescription,
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

const criteria = [
  "Can it directly edit the rendered page, or only annotate and inspect?",
  "What changes does it capture: styles, text, DOM, comments or screenshots?",
  "How does it hand work to a person or coding agent?",
  "Which browsers, editors and source frameworks constrain the workflow?",
  "Is the product open source, paid, hosted or tied to another platform?",
];

export default function ComparisonsIndex() {
  return (
    <>
      <JsonLd
        data={collectionPageSchema({
          name: pageTitle,
          description: pageDescription,
          url: "https://designmode.app/compare",
          items: comparisons.filter(isComparisonIndexable).map((item) => ({
            name: `Design Mode vs ${item.competitor}`,
            url: `https://designmode.app/compare/${item.slug}`,
          })),
        })}
      />

      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-5xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              Which browser visual editor fits your workflow?
            </h1>
            <p className="text-muted-foreground mt-4 max-w-3xl text-base leading-relaxed">
              There is no single best tool for every job. Choose a direct
              live-page editor when you need to make the intended visual change,
              an annotation tool when feedback is enough, browser automation for
              agent verification, or Figma-to-code software when the design file
              is the source of truth.
            </p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <div className="container max-w-5xl">
          <h2 className="text-2xl tracking-tight md:text-3xl">
            How to evaluate the options
          </h2>
          <p className="text-muted-foreground mt-4 max-w-3xl leading-relaxed">
            Design Mode is strongest when you want to edit a rendered webpage,
            record structured changes, and hand them to an independent coding
            agent. It is not a source-code editor, deployment system or
            substitute for a design file when you need greenfield exploration.
          </p>
          <ul className="text-muted-foreground mt-6 grid gap-4 md:grid-cols-2">
            {criteria.map((criterion) => (
              <li
                key={criterion}
                className="rounded-lg border p-4 leading-relaxed"
              >
                {criterion}
              </li>
            ))}
          </ul>
          <p className="bg-muted text-muted-foreground mt-6 rounded-lg border p-4 text-base leading-relaxed">
            Source-backed comparisons include dated references and explain what
            was documented or tested. Older unverified drafts remain excluded
            from search indexing until their claims have been reviewed.
          </p>
        </div>

        <DashedLine className="container mt-16 max-w-5xl" />
        <div className="container mt-12 grid max-w-5xl gap-6 md:grid-cols-2 lg:grid-cols-3">
          {comparisons.map((comparison) => (
            <Link
              key={comparison.slug}
              href={`/compare/${comparison.slug}`}
              className="group flex flex-col rounded-lg border p-6 transition-shadow hover:shadow-md"
            >
              <span className="text-muted-foreground text-base font-medium tracking-wide uppercase">
                {isComparisonIndexable(comparison)
                  ? "Source-backed comparison"
                  : "Research draft"}
              </span>
              <h2 className="font-display mt-2 text-2xl font-semibold">
                Design Mode vs {comparison.competitor}
              </h2>
              <p className="text-muted-foreground mt-2 text-base leading-relaxed">
                {comparison.oneLiner}
              </p>
              <span className="text-foreground/80 mt-4 inline-flex items-center gap-2 text-base font-medium underline underline-offset-8">
                Read the comparison{" "}
                <ArrowUpRight className="size-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
              </span>
            </Link>
          ))}
        </div>
      </section>
    </>
  );
}
