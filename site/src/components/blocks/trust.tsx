import { Code2, Eye, HardDrive, UserX } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";

const ICONS = [HardDrive, UserX, Eye, Code2] as const;

export function Trust({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].trust;
  return (
    <section id="trust" aria-labelledby="trust-heading" className="py-14 md:py-20">
      <div className="container max-w-5xl">
        <div className="max-w-[720px]">
          <p className="text-muted-foreground text-sm font-medium tracking-wider">{copy.eyebrow}</p>
          <h2 id="trust-heading" className="mt-4 text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mt-4 max-w-[640px] text-base leading-relaxed">{copy.description}</p>
        </div>

        <div className="border-border mt-9 grid gap-x-8 gap-y-6 border-t pt-6 sm:grid-cols-2 lg:grid-cols-4">
          {copy.cards.map((card, index) => {
            const Icon = ICONS[index % ICONS.length];
            return (
              <article key={card.title} className="min-w-0 py-2">
                <span className="bg-accent text-foreground inline-flex size-9 items-center justify-center rounded-lg">
                  <Icon size={18} aria-hidden="true" />
                </span>
                <h3 className="mt-5 text-base font-semibold">{card.title}</h3>
                <p className="text-muted-foreground mt-2 text-sm leading-relaxed">{card.body}</p>
              </article>
            );
          })}
        </div>
      </div>
    </section>
  );
}
