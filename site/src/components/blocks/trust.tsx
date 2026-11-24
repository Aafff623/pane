import { Code2, Eye, HardDrive, UserX } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";

const ICONS = [
  [HardDrive, "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300"],
  [UserX, "bg-violet-100 text-violet-800 dark:bg-violet-950 dark:text-violet-300"],
  [Eye, "bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300"],
  [Code2, "bg-amber-100 text-amber-900 dark:bg-amber-950 dark:text-amber-300"],
] as const;

export function Trust({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].trust;
  return (
    <section id="trust" aria-labelledby="trust-heading" className="py-14 md:py-20">
      <div className="container max-w-5xl">
        <div className="mx-auto max-w-[720px] text-center">
          <p className="text-primary text-sm font-medium tracking-wider">{copy.eyebrow}</p>
          <h2 id="trust-heading" className="mt-4 text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-4 max-w-[640px] leading-relaxed">{copy.description}</p>
        </div>

        <div className="mt-9 grid gap-3 sm:grid-cols-2 md:mt-12 lg:grid-cols-4">
          {copy.cards.map((card, index) => {
            const [Icon, tone] = ICONS[index % ICONS.length];
            return (
              <article key={card.title} className="border-border bg-card rounded-2xl border p-6">
                <span className={`inline-flex size-10 items-center justify-center rounded-xl ${tone}`}>
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
