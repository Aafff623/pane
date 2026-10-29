import Link from "next/link";
import { notFound } from "next/navigation";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import {
  JsonLd,
  breadcrumbSchema,
  howToSchema,
} from "@/components/site/json-ld";
import { RelatedLinks } from "@/components/site/related-links";
import { Button } from "@/components/ui/button";
import { useCaseContextualLinks } from "@/content/contextual-links";
import { getUseCase, useCases } from "@/content/use-cases";

export function generateStaticParams() {
  return useCases.map((u) => ({ slug: u.slug }));
}

export const dynamicParams = false;

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const u = getUseCase(slug);
  if (!u) return {};
  const url = `https://designmode.app/use-cases/${u.slug}`;
  const title =
    u.slug === "ui-testing-export-to-developers"
      ? "Export visual UI bugs with context | Design Mode"
      : `${u.title} | Design Mode`;
  const description = `${u.title}: edit the rendered page, capture the exact changes, and hand a structured specification to a coding agent.`;
  return {
    title: { absolute: title },
    description,
    keywords: u.keywords,
    alternates: { canonical: url },
    openGraph: {
      title,
      description,
      url,
      type: "article",
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

export default async function UseCasePage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const u = getUseCase(slug);
  if (!u) notFound();

  const url = `https://designmode.app/use-cases/${u.slug}`;

  return (
    <>
      <JsonLd
        data={howToSchema({
          name: u.title,
          description: u.intro,
          url,
          steps: u.workflow,
        })}
      />
      <JsonLd
        data={breadcrumbSchema([
          { name: "Home", url: "https://designmode.app/" },
          { name: "Use cases", url: "https://designmode.app/use-cases" },
          { name: u.title, url },
        ])}
      />

      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-4xl">
            <div className="text-muted-foreground mb-4 text-base font-medium tracking-wide uppercase">
              {u.persona}
            </div>
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              {u.title}
            </h1>
            <p className="text-muted-foreground mt-4 text-base">{u.intro}</p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-4xl" />
        <article className="container mt-12 max-w-4xl space-y-12">
          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">The problem</h2>
            <p className="text-muted-foreground mt-4 leading-relaxed">
              {u.problem}
            </p>
          </div>

          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">
              The workflow
            </h2>
            <ol className="mt-6 space-y-6">
              {u.workflow.map((step, i) => (
                <li
                  key={i}
                  id={`step-${i + 1}`}
                  className="border-foreground/10 border-l-2 pl-6"
                >
                  <div className="text-muted-foreground text-base font-medium tracking-wide uppercase">
                    Step {i + 1}
                  </div>
                  <h3 className="font-display mt-2 text-2xl font-semibold">
                    {step.name}
                  </h3>
                  <p className="text-muted-foreground mt-2 leading-relaxed">
                    {step.text}
                  </p>
                </li>
              ))}
            </ol>
          </div>

          <div>
            <h2 className="text-2xl tracking-tight md:text-3xl">The outcome</h2>
            <p className="text-muted-foreground mt-4 leading-relaxed">
              {u.outcome}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-4 pt-4">
            <Button asChild>
              <Link href="/demo">Try the live demo</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/mcp">Set up your AI agent</Link>
            </Button>
          </div>
        </article>

        <RelatedLinks
          title="Continue this workflow"
          links={[
            ...(useCaseContextualLinks[u.slug] ?? []),
            ...u.related
              .map((slug) => getUseCase(slug))
              .filter((r): r is NonNullable<typeof r> => Boolean(r))
              .map((r) => ({
                href: `/use-cases/${r.slug}`,
                title: r.title,
                description: r.intro.split(". ")[0] + ".",
              })),
          ]}
        />
      </section>
    </>
  );
}
