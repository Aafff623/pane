import Link from "next/link";

import { ArrowUpRight } from "lucide-react";

import { DashedLine } from "@/components/dashed-line";

export type RelatedLink = {
  href: string;
  title: string;
  description?: string;
};

export function RelatedLinks({
  title = "Keep reading",
  links,
}: {
  title?: string;
  links: RelatedLink[];
}) {
  if (!links.length) return null;
  return (
    <section className="container mt-20 max-w-5xl">
      <DashedLine />
      <h2 className="mt-12 text-2xl font-semibold tracking-tight md:text-3xl">
        {title}
      </h2>
      <div className="mt-6 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {links.map((l) => (
          <Link
            key={l.href}
            href={l.href}
            className="group rounded-2xl border p-6 transition-shadow hover:shadow-md"
          >
            <div className="flex items-start justify-between gap-4">
              <h3 className="font-display text-base font-semibold">
                {l.title}
              </h3>
              <ArrowUpRight className="text-muted-foreground size-4 shrink-0 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
            </div>
            {l.description && (
              <p className="text-muted-foreground mt-2 text-base leading-relaxed">
                {l.description}
              </p>
            )}
          </Link>
        ))}
      </div>
    </section>
  );
}
