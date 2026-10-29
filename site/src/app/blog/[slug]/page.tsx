import { notFound } from "next/navigation";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import {
  JsonLd,
  blogPostingSchema,
  breadcrumbSchema,
} from "@/components/site/json-ld";
import { RelatedLinks } from "@/components/site/related-links";
import { getPost, posts } from "@/content/blog";
import { blogContextualLinks } from "@/content/contextual-links";

export function generateStaticParams() {
  return posts.map((p) => ({ slug: p.slug }));
}

export const dynamicParams = false;

const socialCopy: Record<string, { title: string; description: string }> = {
  "vibe-coding-visual-editing-workflow": {
    title: "Visual editing in the AI coding loop | Design Mode",
    description:
      "Where browser visual editing fits between an initial coding-agent build and the reviewed source-code correction.",
  },
  "redesigning-a-tailwind-landing-page-with-claude-code": {
    title: "Redesign a Tailwind page with Claude Code | Design Mode",
    description:
      "An illustrative workflow for editing a rendered Tailwind page, briefing Claude Code and reviewing its source changes. Not a measured case study.",
  },
  "design-mode-1-9-0-release": {
    title: "Design Mode 1.9.0 release notes",
    description:
      "Design-system tokens, trigger-first motion controls and the dedicated MCP page introduced in Design Mode 1.9.0.",
  },
  "design-mode-2-0-0-release": {
    title: "Design Mode 2.0.0 release notes",
    description:
      "Firefox support, alignment guides and a compact colour-picker workflow in Design Mode 2.0.0.",
  },
};

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const p = getPost(slug);
  if (!p) return {};
  const url = `https://designmode.app/blog/${p.slug}`;
  const title = socialCopy[p.slug]?.title ?? p.title;
  const description = socialCopy[p.slug]?.description ?? p.excerpt;
  return {
    title: { absolute: title },
    description,
    keywords: p.keywords,
    alternates: { canonical: url },
    openGraph: {
      title,
      description,
      url,
      type: "article",
      publishedTime: p.datePublished,
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

const dateFmt = new Intl.DateTimeFormat("en-US", {
  year: "numeric",
  month: "long",
  day: "numeric",
});

export default async function BlogPostPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const p = getPost(slug);
  if (!p) notFound();

  const url = `https://designmode.app/blog/${p.slug}`;

  return (
    <>
      <JsonLd
        data={blogPostingSchema({
          title: p.metaTitle,
          description: p.metaDescription,
          url,
          datePublished: p.datePublished,
        })}
      />
      <JsonLd
        data={breadcrumbSchema([
          { name: "Home", url: "https://designmode.app/" },
          { name: "Blog", url: "https://designmode.app/blog" },
          { name: p.title, url },
        ])}
      />

      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-4xl">
            <time
              dateTime={p.datePublished}
              className="text-muted-foreground text-base font-medium tracking-wide uppercase"
            >
              {dateFmt.format(new Date(p.datePublished))}
            </time>
            <h1 className="mt-4 text-3xl tracking-tight sm:text-4xl md:text-5xl">
              {p.title}
            </h1>
            <p className="text-muted-foreground mt-4 text-base">{p.excerpt}</p>
          </div>
        </section>
      </Background>

      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-4xl" />
        <article className="container mt-12 max-w-4xl space-y-10">
          {p.body.map((block, i) => (
            <div key={i}>
              {block.heading && (
                <h2 className="text-2xl tracking-tight md:text-3xl">
                  {block.heading}
                </h2>
              )}
              <div className="text-muted-foreground mt-4 space-y-4 leading-relaxed">
                {block.paragraphs.map((para, j) => (
                  <p key={j}>{para}</p>
                ))}
              </div>
            </div>
          ))}
        </article>

        <RelatedLinks
          title="Keep reading"
          links={[
            ...(blogContextualLinks[p.slug] ?? []),
            ...p.related
              .map((slug) => getPost(slug))
              .filter((r): r is NonNullable<typeof r> => Boolean(r))
              .map((r) => ({
                href: `/blog/${r.slug}`,
                title: r.title,
                description: r.excerpt,
              })),
            {
              href: "/docs/mcp-setup",
              title: "Use the current MCP setup guide",
              description:
                "Version-stamped Claude Code and Cursor configuration and verification.",
            },
            {
              href: "/features",
              title: "Review the current feature surface",
              description:
                "Layers, visual controls, Changes and the two hand-off methods.",
            },
          ]}
        />
      </section>
    </>
  );
}
