import Link from "next/link";

import { messages, type Lang } from "@/components/i18n";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";

export function FAQ({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].faq;
  return (
    <section id="faq" className="py-14 md:py-20">
      <div className="container grid max-w-5xl gap-6 md:grid-cols-[.8fr_1.2fr] md:gap-x-14">
        <div className="space-y-4">
          {/* The eyebrow is what makes this section read as a peer of the two
              above it instead of jumping straight to a headline. */}
          <p className="text-muted-foreground text-sm font-medium tracking-wider">{copy.eyebrow}</p>
          <h2 className="mt-4 text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">{copy.title}</h2>
          <p className="text-muted-foreground mt-4 max-w-[640px] text-base leading-relaxed">{copy.description}</p>
        </div>

        <Accordion type="single" collapsible className="mx-auto w-full max-w-4xl">
          {copy.items.map((item, index) => (
            <AccordionItem key={item.q} value={`faq-${index}`}>
              <AccordionTrigger className="text-left">{item.q}</AccordionTrigger>
              <AccordionContent className="text-muted-foreground leading-relaxed">{item.a}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>

        <p className="text-muted-foreground text-sm md:col-start-2">
          {copy.more}{" "}
          <Link href="https://github.com/Aafff623/pane/issues" className="underline underline-offset-8" target="_blank" rel="noopener noreferrer">
            {copy.moreLink}
          </Link>
          .
        </p>
      </div>
    </section>
  );
}
