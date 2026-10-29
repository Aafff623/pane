import Link from "next/link";
import { notFound } from "next/navigation";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import {
  JsonLd,
  breadcrumbSchema,
  articleSchema,
} from "@/components/site/json-ld";
import { RelatedLinks } from "@/components/site/related-links";
import { docs, getDoc } from "@/content/docs";

export function generateStaticParams() {
  return docs.map((d) => ({ slug: d.slug }));
}

export const dynamicParams = false;

const docDescriptions: Record<string, string> = {
  "browser-support":
    "Install Design Mode from the Chrome Web Store or Firefox Add-ons, or add the Safari Web Inspector zip from the latest GitHub release.",
  "mcp-setup":
    "Bearer-token Cloud, Local and Self-hosted MCP setup for Claude Code, Cursor and VS Code, with protected configuration and read-only verification.",
  troubleshooting:
    "Fix common Design Mode installation, side-panel, editing, clipboard and MCP issues in Chrome, Chromium browsers and Firefox.",
};

const docTitles: Record<string, string> = {
  "mcp-setup": "MCP setup for coding agents | Design Mode",
};

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const d = getDoc(slug);
  if (!d) return {};
  const url = `https://designmode.app/docs/${d.slug}`;
  const title = docTitles[d.slug] ?? `${d.title} | Design Mode`;
  const description = docDescriptions[d.slug] ?? d.intro;
  return {
    title: { absolute: title },
    description,
    keywords: d.keywords,
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

export default async function DocPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const d = getDoc(slug);
  if (!d) notFound();

  const url = `https://designmode.app/docs/${d.slug}`;

  return (
    <>
      <JsonLd
        data={articleSchema({
          title: d.title,
          description: d.intro,
          url,
          type: "TechArticle",
        })}
      />
      <JsonLd
        data={breadcrumbSchema([
          { name: "Home", url: "https://designmode.app/" },
          { name: "Docs", url: "https://designmode.app/docs" },
          { name: d.title, url },
        ])}
      />

      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-4xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              {d.title}
            </h1>
            <p className="text-muted-foreground mt-4 text-base">{d.intro}</p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-4xl" />
        <article className="container mt-12 max-w-4xl space-y-10 break-words">
          {d.sections.map((s, i) => (
            <div key={i} id={`step-${i + 1}`}>
              <h2 className="text-2xl tracking-tight md:text-3xl">
                {s.heading}
              </h2>
              <p className="text-muted-foreground mt-4 leading-relaxed">
                {s.body}
              </p>
              {s.links && (
                <ul className="mt-4 space-y-2">
                  {s.links.map((link) => (
                    <li key={link.href}>
                      <Link
                        href={link.href}
                        className="underline underline-offset-4"
                      >
                        {link.label}
                      </Link>
                    </li>
                  ))}
                </ul>
              )}
              {s.code && (
                <pre className="bg-muted text-foreground mt-4 overflow-x-auto rounded-lg border p-4 text-base leading-relaxed">
                  <code>{s.code}</code>
                </pre>
              )}
            </div>
          ))}
        </article>

        <RelatedLinks
          title="Related docs"
          links={[
            ...d.related
              .map((slug) => getDoc(slug))
              .filter((r): r is NonNullable<typeof r> => Boolean(r))
              .map((r) => ({
                href: `/docs/${r.slug}`,
                title: r.title,
                description: r.intro,
              })),
            {
              href: "/demo",
              title: "Try the editing workflow",
              description:
                "Inspect the demo, make a harmless change and copy the specification.",
            },
            {
              href: "/mcp",
              title: "Understand the agent hand-off",
              description:
                "Choose Copy as Prompt or a Cloud, Local or Self-hosted MCP connection.",
            },
          ]}
        />
      </section>
    </>
  );
}
