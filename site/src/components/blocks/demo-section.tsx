"use client";

import { useEffect, useState } from "react";
import { ArrowSquareOut, ArrowsClockwise } from "@phosphor-icons/react";

import { Button } from "@/components/ui/button";
import type { Dictionary, Lang } from "@/i18n";

export function DemoSection({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  const [nonce, setNonce] = useState(0);
  const [theme, setTheme] = useState<"dark" | "light">("dark");

  useEffect(() => {
    const sync = () => setTheme(document.documentElement.classList.contains("dark") ? "dark" : "light");
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  const src = `/demo/index.html?theme=${theme}&lang=${lang}`;

  return (
    <section id="demo" className="py-16 md:py-24 lg:py-32" aria-labelledby="demo-title">
      <div className="container">
        <div className="mx-auto max-w-[820px] text-center">
          <p className="text-muted-foreground text-sm font-medium tracking-[0.18em] uppercase">{dict.demo.eyebrow}</p>
          <h2 id="demo-title" className="mt-6 text-2xl tracking-tight md:text-4xl lg:text-5xl">
            {dict.demo.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] leading-relaxed">{dict.demo.description}</p>
        </div>

        <div className="border-border bg-card mt-12 overflow-hidden rounded-2xl border shadow-lg">
          <div className="border-border flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
            <span className="inline-flex items-center gap-2 text-xs font-medium tracking-widest text-primary uppercase">
              <i className="bg-primary inline-block size-1.5 rounded-full" aria-hidden="true" />
              {dict.demo.note}
            </span>
            <div className="flex items-center gap-2">
              <Button variant="ghost" size="sm" onClick={() => setNonce((value) => value + 1)}>
                <ArrowsClockwise size={15} aria-hidden="true" />
                {dict.demo.reset}
              </Button>
              <Button asChild variant="outline" size="sm">
                <a href={src} target="_blank" rel="noopener noreferrer">
                  <ArrowSquareOut size={15} aria-hidden="true" />
                  {dict.demo.open}
                </a>
              </Button>
            </div>
          </div>
          <div className="bg-muted/40 flex justify-center px-4 py-8 sm:py-12">
            <div className="w-full max-w-[420px] overflow-hidden rounded-xl border border-border bg-background shadow-xl">
              <iframe
                key={`${nonce}-${theme}-${lang}`}
                src={src}
                title={dict.demo.iframeTitle}
                loading="lazy"
                className="h-[620px] w-full max-w-full"
              />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
