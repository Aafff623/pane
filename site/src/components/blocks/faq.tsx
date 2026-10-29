import Link from "next/link";

import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import type { Dictionary } from "@/i18n";

export function FAQ({ dict }: { dict: Dictionary }) {
  return (
    <section id="faq" className="py-16 md:py-24 lg:py-32">
      <div className="container max-w-5xl">
        <div className="mb-12 space-y-4 text-center lg:mb-16">
          <h2 className="text-2xl tracking-tight md:text-4xl lg:text-5xl">{dict.faq.title}</h2>
          <p className="text-muted-foreground mx-auto max-w-2xl leading-relaxed">{dict.faq.description}</p>
        </div>

        <Accordion type="single" collapsible className="mx-auto w-full max-w-4xl">
          {dict.faq.items.map((item, index) => (
            <AccordionItem key={item.q} value={`faq-${index}`}>
              <AccordionTrigger className="text-left">{item.q}</AccordionTrigger>
              <AccordionContent className="text-muted-foreground leading-relaxed">{item.a}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>

        <p className="text-muted-foreground mt-10 text-center text-base">
          {dict.faq.more}{" "}
          <Link href="https://github.com/Aafff623/pane/issues" className="underline underline-offset-8" target="_blank" rel="noopener noreferrer">
            {dict.faq.moreLink}
          </Link>
          .
        </p>
      </div>
    </section>
  );
}
