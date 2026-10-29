"use client";

import Image from "next/image";
import { useState } from "react";

import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import type { Dictionary } from "@/i18n";
import styles from "./panel-anatomy.module.css";

const MARKS = [
  { left: "17%", top: "62%" },
  { left: "84%", top: "62%" },
  { left: "42%", top: "75%" },
  { left: "50%", top: "32%" },
  { left: "88%", top: "6%" },
  { left: "50%", top: "97%" },
];

export function PanelAnatomy({ dict }: { dict: Dictionary }) {
  const [active, setActive] = useState("0");

  return (
    <section id="panel-anatomy" aria-labelledby="panel-anatomy-title" className="py-16 md:py-24 lg:py-32">
      <div className="container">
        <div className="mx-auto max-w-[820px] text-center">
          <p className="text-muted-foreground text-sm font-medium tracking-[0.18em] uppercase">{dict.panel.eyebrow}</p>
          <h2 id="panel-anatomy-title" className="mt-6 text-2xl tracking-tight text-balance md:text-4xl lg:text-5xl">
            {dict.panel.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[620px] leading-relaxed">{dict.panel.description}</p>
        </div>

        <div className="mt-14 grid gap-10 lg:mt-20 lg:grid-cols-[minmax(0,360px)_minmax(0,1fr)] lg:items-start lg:gap-16">
          <Accordion
            type="single"
            collapsible
            value={active}
            onValueChange={(value) => setActive(value || "0")}
            className="w-full"
          >
            {dict.panel.stages.map((stage, index) => (
              <AccordionItem key={stage.title} value={String(index)}>
                <AccordionTrigger className="text-left">
                  <span className="flex items-center gap-4">
                    <span className="text-muted-foreground text-sm tabular-nums" aria-hidden="true">
                      0{index + 1}
                    </span>
                    {stage.title}
                  </span>
                </AccordionTrigger>
                <AccordionContent className="text-muted-foreground pl-8 leading-relaxed">{stage.body}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>

          <figure className={styles.figure}>
            <div className={styles.frame}>
              <Image
                src="/img/pane-panel-dark.png"
                alt={dict.panel.imageAlt}
                width={1600}
                height={1000}
                unoptimized
                className="hidden h-auto w-full dark:block"
              />
              <Image
                src="/img/pane-panel-light.png"
                alt={dict.panel.imageAlt}
                width={1600}
                height={1000}
                unoptimized
                className="block h-auto w-full dark:hidden"
              />
              {MARKS.map((mark, index) => {
                const on = String(index) === active;
                return (
                  <span
                    key={index}
                    className={`${styles.marker} ${on ? styles.markerActive : ""}`}
                    style={{ left: mark.left, top: mark.top }}
                    aria-hidden="true"
                  >
                    <b>{index + 1}</b>
                    {on ? <em>{dict.panel.stages[index].title}</em> : null}
                  </span>
                );
              })}
            </div>
            <figcaption className="text-muted-foreground mt-3 text-sm">{dict.panel.hint}</figcaption>
          </figure>
        </div>
      </div>
    </section>
  );
}
