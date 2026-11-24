import Link from "next/link";

import { messages, type Lang } from "@/components/i18n";
import { Button } from "@/components/ui/button";
import { SmartDownload } from "@/components/smart-download";

export function ClosingCta({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].closing;
  const demoHref = lang === "zh" ? "/zh/demo" : "/demo";

  return (
    <section aria-labelledby="closing-cta-heading" className="py-13 md:py-18">
      <div className="container text-center">
        <p className="text-primary text-sm font-medium tracking-wider">{copy.eyebrow}</p>
        <h2
          id="closing-cta-heading"
          className="mx-auto mt-4 max-w-4xl whitespace-pre-line text-3xl leading-[1.1] tracking-tight text-balance md:text-4xl lg:text-5xl"
        >
          {copy.title}
        </h2>
        <p className="text-muted-foreground mx-auto mt-4 max-w-xl text-base leading-relaxed">{copy.description}</p>
        <div className="mt-7 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <SmartDownload label={copy.primary} macLabel={copy.macPrimary} className="h-14 max-sm:w-full max-sm:max-w-xs" />
          <Button asChild variant="outline" size="lg" className="h-14 max-sm:w-full max-sm:max-w-xs">
            <Link href={demoHref}>{copy.secondary}</Link>
          </Button>
        </div>
      </div>
    </section>
  );
}
