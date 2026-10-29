import Link from "next/link";
import type { ReactNode } from "react";

import { SiteShell } from "@/components/site-shell";
import { SITE_FACTS } from "@/content/facts";
import { dictionaries, type Lang } from "@/i18n";

export function LegalPage({
  lang,
  title,
  updated,
  authoritativeHref,
  authoritativeLabel,
  alternate,
  children,
}: {
  lang: Lang;
  title: string;
  updated: string;
  authoritativeHref: string;
  authoritativeLabel: string;
  alternate: { href: string; label: string };
  children: ReactNode;
}) {
  const dict = dictionaries[lang];
  return (
    <SiteShell lang={lang}>
      <article className="container max-w-3xl py-16 md:py-24">
        <h1 className="text-3xl tracking-tight md:text-4xl">{title}</h1>
        <p className="text-muted-foreground mt-3 text-sm">
          {updated} · {authoritativeLabel}{" "}
          <a className="underline underline-offset-4" href={authoritativeHref} target="_blank" rel="noopener noreferrer">
            {authoritativeHref.replace(SITE_FACTS.repo + "/blob/main/", "")}
          </a>{" "}
          ·{" "}
          <Link className="underline underline-offset-4" href={alternate.href}>
            {alternate.label}
          </Link>
        </p>
        <div className="legal mt-10">{children}</div>
      </article>
    </SiteShell>
  );
}
