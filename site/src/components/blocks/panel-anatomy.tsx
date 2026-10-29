"use client";

import { useState } from "react";

import { messages, type Lang } from "@/components/i18n";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import styles from "./panel-anatomy.module.css";

const SHOTS = {
  light: "/img/pane-panel-light.png",
  dark: "/img/pane-panel-dark.png",
};

export function PanelAnatomy({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].panel;
  const [active, setActive] = useState(copy.items[0].title);

  return (
    <section id="panel-anatomy" aria-labelledby="panel-anatomy-title" className="py-16 md:py-24 lg:py-32">
      <div className={styles.track}>
        <div className={`container ${styles.sticky}`}>
          <h2 id="panel-anatomy-title" className="text-center text-3xl tracking-tight text-balance sm:text-4xl md:text-5xl lg:text-6xl">
            {copy.title}
          </h2>

          <div className={styles.walkthrough}>
            <nav className={styles.dots} aria-label="Panel anatomy stages">
              {copy.items.map((item) => (
                <button
                  key={item.title}
                  type="button"
                  aria-label={item.title}
                  aria-current={active === item.title ? "step" : undefined}
                  onClick={() => setActive(item.title)}
                >
                  <span />
                </button>
              ))}
            </nav>

            <Accordion type="single" value={active} onValueChange={(value) => value && setActive(value)} className={styles.accordion}>
              {copy.items.map((item, index) => (
                <AccordionItem key={item.title} value={item.title}>
                  <AccordionTrigger className="focus-visible:ring-ring gap-4 rounded-sm py-3 text-base focus-visible:ring-2 focus-visible:outline-none motion-reduce:transition-none motion-reduce:[&>svg]:transition-none">
                    <span className="flex items-center gap-4">
                      <span aria-hidden="true" className="text-muted-foreground text-base tabular-nums">
                        0{index + 1}
                      </span>
                      {item.title}
                    </span>
                  </AccordionTrigger>
                  <AccordionContent className="text-muted-foreground min-h-18 pl-8 text-base leading-relaxed">
                    {item.lines.map((line) => (
                      <p key={line}>{line}</p>
                    ))}
                  </AccordionContent>
                </AccordionItem>
              ))}
            </Accordion>

            <figure className={styles.figure}>
              <div className={styles.scene}>
                <div className={styles.replica} role="img" aria-label={copy.title}>
                  <img src={SHOTS.light} alt="" className="block h-auto w-full dark:hidden" />
                  <img src={SHOTS.dark} alt="" className="hidden h-auto w-full dark:block" />
                </div>
              </div>
            </figure>
          </div>
        </div>
      </div>
    </section>
  );
}
