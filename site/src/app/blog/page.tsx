import Link from "next/link";

import { ArrowUpRight } from "lucide-react";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import { JsonLd, collectionPageSchema } from "@/components/site/json-ld";
import { posts } from "@/content/blog";

export const metadata = {
  title: { absolute: "Build notes and visual editing guides | Design Mode" },
  description:
    "Stories from building Design Mode and walkthroughs of real workflows: MCP architecture, vibe coding loops, changelog deep-dives, Tailwind redesigns with Claude Code.",
  keywords: [
    "Design Mode blog",
    "vibe coding articles",
    "MCP articles",
    "Claude Code blog",
    "AI design agent",
  ],
  alternates: { canonical: "https://designmode.app/blog" },
  openGraph: {
    type: "website",
    title: "Build notes and visual editing guides | Design Mode",
    description:
      "Stories from building Design Mode and walkthroughs of real workflows: MCP architecture, vibe coding loops, changelog deep-dives, Tailwind redesigns with Claude Code.",
    url: "https://designmode.app/blog",
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
    title: "Build notes and visual editing guides | Design Mode",
    description:
      "Stories from building Design Mode and walkthroughs of real workflows: MCP architecture, vibe coding loops, changelog deep-dives, Tailwind redesigns with Claude Code.",
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

const dateFmt = new Intl.DateTimeFormat("en-US", {
  year: "numeric",
  month: "long",
  day: "numeric",
});

export default function BlogIndex() {
  const sorted = [...posts].sort((a, b) =>
    b.datePublished.localeCompare(a.datePublished),
  );

  return (
    <>
      <JsonLd
        data={collectionPageSchema({
          name: "Design Mode build notes and guides",
          description:
            "Build notes and workflow guides for browser visual editing and coding-agent hand-off.",
          url: "https://designmode.app/blog",
          items: sorted.map((post) => ({
            name: post.title,
            url: `https://designmode.app/blog/${post.slug}`,
          })),
        })}
      />
      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-5xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              Blog
            </h1>
            <p className="text-muted-foreground mt-4 max-w-3xl text-base">
              Build notes from Design Mode and walkthroughs of real workflows —
              MCP architecture, vibe coding, Tailwind redesigns, changelog
              deep-dives.
            </p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-5xl" />
        <div className="container mt-12 max-w-5xl space-y-6">
          {sorted.map((p) => (
            <Link
              key={p.slug}
              href={`/blog/${p.slug}`}
              className="group block rounded-2xl border p-6 transition-shadow hover:shadow-md"
            >
              <time
                dateTime={p.datePublished}
                className="text-muted-foreground text-base font-medium tracking-wide uppercase"
              >
                {dateFmt.format(new Date(p.datePublished))}
              </time>
              <h2 className="font-display mt-2 text-2xl leading-snug font-semibold md:text-3xl">
                {p.title}
              </h2>
              <p className="text-muted-foreground mt-2 leading-relaxed">
                {p.excerpt}
              </p>
              <span className="text-foreground/80 mt-4 inline-flex items-center gap-2 text-base font-medium underline underline-offset-8">
                Read{" "}
                <ArrowUpRight className="size-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
              </span>
            </Link>
          ))}
        </div>
      </section>
    </>
  );
}
