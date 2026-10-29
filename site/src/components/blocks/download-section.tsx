import Link from "next/link";

import { DownloadButton } from "@/components/download-button";
import { Button } from "@/components/ui/button";
import { SITE_FACTS } from "@/content/facts";
import type { Dictionary } from "@/i18n";

export function DownloadSection({ dict }: { dict: Dictionary }) {
  return (
    <section id="download" className="py-16 md:py-24 lg:py-32">
      <div className="container">
        <div className="mx-auto max-w-[760px] text-center">
          <p className="text-muted-foreground text-sm font-medium tracking-[0.18em] uppercase">{dict.download.eyebrow}</p>
          <h2 className="mt-6 text-2xl tracking-tight md:text-4xl lg:text-5xl">{dict.download.title}</h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[620px] leading-relaxed">{dict.download.description}</p>
        </div>

        <div className="mt-12 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <DownloadButton
            labels={{ windows: dict.download.windows, macos: dict.download.macos }}
            size="lg"
            className="h-14 w-full sm:w-auto"
          />
          <Button asChild variant="outline" size="lg" className="h-14 w-full sm:w-auto">
            <a href={SITE_FACTS.releases} target="_blank" rel="noopener noreferrer">
              {dict.download.macos}
            </a>
          </Button>
          <Button variant="ghost" size="lg" className="h-14 w-full cursor-not-allowed sm:w-auto" disabled aria-disabled="true">
            {dict.download.linux}
          </Button>
        </div>

        <dl className="mx-auto mt-16 grid max-w-4xl gap-6 sm:grid-cols-3">
          {dict.download.facts.map((fact) => (
            <div key={fact.title} className="border-border rounded-xl border p-6">
              <dt className="text-foreground text-base font-semibold">{fact.title}</dt>
              <dd className="text-muted-foreground mt-2 text-sm leading-relaxed">{fact.body}</dd>
            </div>
          ))}
        </dl>

        <div className="text-muted-foreground mx-auto mt-12 max-w-3xl space-y-3 text-center text-sm">
          <p>
            <span className="text-foreground font-medium">{dict.download.versionLabel}</span>{" "}
            <b className="text-foreground tabular-nums">v{SITE_FACTS.version}</b> · {dict.download.platform}
          </p>
          <p className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1">
            <span className="text-foreground font-medium">{dict.download.winget}</span>
            <code className="bg-muted rounded px-2 py-0.5 text-xs">{SITE_FACTS.winget}</code>
          </p>
          <p className="flex flex-wrap items-center justify-center gap-x-4 gap-y-1">
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.mirror} target="_blank" rel="noopener noreferrer">
              {dict.download.mirror}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.repo} target="_blank" rel="noopener noreferrer">
              {dict.download.source}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.license} target="_blank" rel="noopener noreferrer">
              {dict.download.license}
            </a>
          </p>
        </div>
      </div>
    </section>
  );
}
