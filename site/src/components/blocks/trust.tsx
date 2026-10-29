import { Code2, Eye, HardDrive, UserX } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";

const ICONS = [HardDrive, UserX, Eye, Code2];

export function Trust({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].trust;
  return (
    <section id="trust" aria-labelledby="trust-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container max-w-5xl">
        <div className="mx-auto max-w-[720px] text-center">
          <p className="text-muted-foreground text-base font-medium tracking-widest uppercase">{copy.eyebrow}</p>
          <h2 id="trust-heading" className="mt-6 text-2xl tracking-tight md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] leading-relaxed">{copy.description}</p>
        </div>

        <div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
          {copy.cards.map((card, index) => {
            const Icon = ICONS[index] ?? HardDrive;
            return (
              <article key={card.title} className="border-border bg-card rounded-2xl border p-6">
                <span className="bg-accent text-primary inline-flex size-10 items-center justify-center rounded-xl">
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
