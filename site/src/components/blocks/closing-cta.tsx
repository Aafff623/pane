import Link from "next/link";

import { DownloadButton } from "@/components/download-button";
import { Button } from "@/components/ui/button";
import type { Dictionary, Lang } from "@/i18n";

export function ClosingCta({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  const base = lang === "zh" ? "" : "/en";
  return (
    <section aria-labelledby="closing-cta-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container text-center">
        <p className="text-muted-foreground text-sm font-medium tracking-[0.18em] uppercase">{dict.closing.eyebrow}</p>
        <h2
          id="closing-cta-heading"
          className="mx-auto mt-8 max-w-4xl text-5xl leading-[1.05] tracking-tight whitespace-pre-line sm:text-6xl lg:text-[72px]"
        >
          {dict.closing.title}
        </h2>
        <p className="text-muted-foreground mx-auto mt-8 max-w-xl text-base leading-relaxed">{dict.closing.description}</p>
        <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <DownloadButton
            labels={{ windows: dict.closing.download, macos: dict.download.macos }}
            size="lg"
            className="h-14 max-sm:w-full max-sm:max-w-xs"
          />
          <Button asChild variant="outline" size="lg" className="h-14 max-sm:w-full max-sm:max-w-xs">
            <Link href={`${base}#demo`}>{dict.closing.demo}</Link>
          </Button>
        </div>
      </div>
    </section>
  );
}
