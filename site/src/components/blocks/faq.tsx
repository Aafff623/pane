import Link from "next/link";

import { messages, type Lang } from "@/components/i18n";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";

export function FAQ({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].faq;
  return (
    <section id="faq" className="py-14 md:py-20">
      <div className="container max-w-5xl">
        <div className="mb-9 space-y-4 text-center md:mb-12">
          <h2 className="text-3xl tracking-tight md:text-4xl lg:text-5xl">{copy.title}</h2>
          <p className="text-muted-foreground mx-auto max-w-2xl leading-relaxed">{copy.description}</p>
        </div>

        <Accordion type="single" collapsible className="mx-auto w-full max-w-4xl">
          {copy.items.map((item, index) => (
            <AccordionItem key={item.q} value={`faq-${index}`}>
              <AccordionTrigger className="text-left">{item.q}</AccordionTrigger>
              <AccordionContent className="text-muted-foreground leading-relaxed">{item.a}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>

        <p className="text-muted-foreground mt-10 text-center text-base">
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
