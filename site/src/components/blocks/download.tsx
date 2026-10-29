import { messages, type Lang } from "@/components/i18n";
import { Button } from "@/components/ui/button";
import { SmartDownload } from "@/components/smart-download";

const RELEASES = "https://github.com/Aafff623/pane/releases/latest";
const MIRROR = "https://gitee.com/fan-tengda/pane/releases";
const REPO = "https://github.com/Aafff623/pane";
const LICENSE = "https://github.com/Aafff623/pane/blob/main/LICENSE";
const VERSION = "0.12.0";

export function DownloadSection({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].download;
  return (
    <section id="download" aria-labelledby="download-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container">
        <div className="mx-auto max-w-[760px] text-center">
          <p className="text-muted-foreground text-base font-medium tracking-widest uppercase">{copy.eyebrow}</p>
          <h2 id="download-heading" className="mt-6 text-2xl tracking-tight md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[620px] leading-relaxed">{copy.description}</p>
        </div>

        <div className="mt-12 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <SmartDownload label={copy.windows} macLabel={copy.macos} className="h-14 w-full sm:w-auto" />
          <Button asChild variant="outline" size="lg" className="h-14 w-full sm:w-auto">
            <a href={RELEASES} target="_blank" rel="noopener noreferrer">
              {copy.allReleases}
            </a>
          </Button>
          <Button variant="ghost" size="lg" className="h-14 w-full cursor-not-allowed sm:w-auto" disabled>
            {copy.linux}
          </Button>
        </div>
        <p className="text-muted-foreground mx-auto mt-4 max-w-[620px] text-center text-sm">{copy.macNote}</p>

        <dl className="mx-auto mt-16 grid max-w-4xl gap-6 sm:grid-cols-3">
          {copy.facts.map((fact) => (
            <div key={fact.title} className="border-border rounded-xl border p-6">
              <dt className="text-base font-semibold">{fact.title}</dt>
              <dd className="text-muted-foreground mt-2 text-sm leading-relaxed">{fact.body}</dd>
            </div>
          ))}
        </dl>

        <div className="text-muted-foreground mx-auto mt-12 max-w-3xl space-y-3 text-center text-sm">
          <p>
            <span className="text-foreground font-medium">{copy.versionLabel}</span>{" "}
            <b className="text-foreground tabular-nums">v{VERSION}</b> · {copy.platform}
          </p>
          <p className="flex flex-wrap items-center justify-center gap-x-3 gap-y-1">
            <span className="text-foreground font-medium">{copy.wingetLabel}</span>
            <code className="bg-muted rounded px-2 py-0.5 text-xs">winget install Aafff623.Pane</code>
          </p>
          <p className="flex flex-wrap items-center justify-center gap-x-4 gap-y-1">
            <a className="hover:text-foreground underline underline-offset-4" href={MIRROR} target="_blank" rel="noopener noreferrer">
              {copy.mirror}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={REPO} target="_blank" rel="noopener noreferrer">
              {copy.source}
            </a>
            <a className="hover:text-foreground underline underline-offset-4" href={LICENSE} target="_blank" rel="noopener noreferrer">
              {copy.license}
            </a>
          </p>
        </div>
      </div>
    </section>
  );
}
