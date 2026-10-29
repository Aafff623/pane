import Link from "next/link";

import { HeroHeadline } from "@/components/blocks/hero-headline";
import { messages, type Lang } from "@/components/i18n";
import { Button } from "@/components/ui/button";

export const Hero = ({ lang = "en" }: { lang?: Lang }) => {
  const copy = messages[lang].hero;
  const demoHref = lang === "zh" ? "/zh/demo" : "/demo";

  return (
    <section className="py-16 md:py-24 lg:py-32">
      <div className="container max-w-[1200px] text-center max-sm:px-4">
        <HeroHeadline lead={copy.lead} before={copy.before} after={copy.after} />

        <p className="text-muted-foreground mx-auto mt-8 max-w-[848px] text-base">{copy.description}</p>

        <div className="mt-16 grid grid-cols-2 items-stretch gap-2 sm:flex sm:flex-wrap sm:items-center sm:justify-center sm:gap-4">
          <Button size="lg" className="h-14 max-sm:w-full max-sm:px-2 max-sm:py-1 max-sm:whitespace-normal text-base" asChild>
            <a href="/download/windows">{copy.primary}</a>
          </Button>
          <Button variant="outline" size="lg" className="h-14 max-sm:w-full max-sm:px-2 max-sm:py-1 max-sm:whitespace-normal text-base" asChild>
            <Link href={demoHref}>{copy.secondary}</Link>
          </Button>
        </div>

        <figure className="mx-auto mt-10 w-full max-w-[980px] sm:mt-16">
          <div className="border-border bg-muted/40 text-muted-foreground flex aspect-[8/5] w-full items-center justify-center rounded-2xl border text-sm">
            {copy.placeholder}
          </div>
          <figcaption className="text-muted-foreground mt-3 text-base">{copy.caption}</figcaption>
        </figure>
      </div>
    </section>
  );
};
