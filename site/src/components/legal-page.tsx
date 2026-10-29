import Link from "next/link";
import type { ReactNode } from "react";

export function LegalPage({
  title,
  updated,
  authoritativeLabel,
  authoritativeHref,
  alternate,
  children,
}: {
  title: string;
  updated: string;
  authoritativeLabel: string;
  authoritativeHref: string;
  alternate: { href: string; label: string };
  children: ReactNode;
}) {
  return (
    <article className="container max-w-3xl py-16 md:py-24">
      <h1 className="text-3xl tracking-tight md:text-4xl">{title}</h1>
      <p className="text-muted-foreground mt-3 text-sm">
        {updated} · {authoritativeLabel}{" "}
        <a className="underline underline-offset-4" href={authoritativeHref} target="_blank" rel="noopener noreferrer">
          {authoritativeHref.split("/").pop()}
        </a>{" "}
        ·{" "}
        <Link className="underline underline-offset-4" href={alternate.href}>
          {alternate.label}
        </Link>
      </p>
      <div className="prose mt-10">{children}</div>
    </article>
  );
}
