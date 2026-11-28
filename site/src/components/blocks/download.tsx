import { messages, type Lang } from "@/components/i18n";
import { Button } from "@/components/ui/button";
import { SmartDownload } from "@/components/smart-download";
import { SITE_FACTS } from "@/content/facts";

const WINGET_SUBMISSION = "https://github.com/microsoft/winget-pkgs/pull/463652";

export function DownloadSection({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].download;
  return (
    <section id="download" aria-labelledby="download-heading" className="py-14 md:py-20">
      <div className="container">
        <div className="mx-auto max-w-[760px] text-center">
          <p className="text-muted-foreground text-sm font-medium tracking-wider">{copy.eyebrow}</p>
          <h2 id="download-heading" className="mt-4 text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-4 max-w-[620px] text-base leading-relaxed">{copy.description}</p>
        </div>

        <div className="mt-12 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <SmartDownload label={copy.windows} macLabel={copy.macos} className="h-14 w-full sm:w-auto" />
          <Button asChild variant="outline" size="lg" className="h-14 w-full sm:w-auto">
            <a href={SITE_FACTS.releases} target="_blank" rel="noopener noreferrer">
              {copy.allReleases}
            </a>
          </Button>
        </div>

        <dl className="mx-auto mt-10 grid max-w-4xl gap-3 sm:grid-cols-3">
          {copy.facts.map((fact) => (
            <div key={fact.title} className="border-border bg-card rounded-xl border p-5">
              <dt className="text-base font-semibold">{fact.title}</dt>
              <dd className="text-muted-foreground mt-2 text-sm leading-relaxed">{fact.body}</dd>
            </div>
          ))}
        </dl>

        <div className="text-muted-foreground mx-auto mt-12 max-w-3xl space-y-3 text-center text-sm">
          <p>
            <span className="text-foreground font-medium">{copy.versionLabel}</span>{" "}
            <b className="text-foreground tabular-nums">v{SITE_FACTS.version}</b>
          </p>
          <p className="flex flex-wrap items-center justify-center gap-x-4 gap-y-1">
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.mirror} target="_blank" rel="noopener noreferrer">
              {copy.mirror}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.repo} target="_blank" rel="noopener noreferrer">
              {copy.source}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={SITE_FACTS.license} target="_blank" rel="noopener noreferrer">
              {copy.license}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={WINGET_SUBMISSION} target="_blank" rel="noopener noreferrer">
              {copy.wingetReview}
            </a>
          </p>
        </div>
      </div>
    </section>
  );
}
