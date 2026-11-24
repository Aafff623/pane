"use client";

import { useRef, useState } from "react";

import { ChevronDown, Keyboard } from "lucide-react";

import { PaneDemoFrame } from "@/components/pane-demo-frame";
import { messages, type Lang } from "@/components/i18n";
import { DEMO_GUIDE, type DemoGuide } from "@/content/demo-guide";
import styles from "./panel-stage.module.css";

export function PanelAnatomy({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].panel;
  const [selection, setSelection] = useState<{ index: number; step: DemoGuide }>({ index: 0, step: DEMO_GUIDE[0] });
  const preview = useRef<HTMLElement>(null);
  const active = selection.index;

  function select(index: number) {
    setSelection({ index, step: DEMO_GUIDE[index] });
    const stage = preview.current;
    if (stage && matchMedia("(max-width: 1023px)").matches && stage.getBoundingClientRect().top > innerHeight * 0.55) {
      stage.scrollIntoView({ block: "start", behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth" });
    }
  }

  return (
    <section id="panel-anatomy" aria-labelledby="panel-anatomy-title" className="py-14 md:py-20">
      <div className="container">
        <h2 id="panel-anatomy-title" className="text-center text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">
          {copy.title}
        </h2>

        <div className={styles.walkthrough}>
          <div className={styles.list}>
            {copy.items.map((item, index) => {
              const open = index === active;
              return (
                <div key={item.title} className={styles.item} data-open={open}>
                  <button
                    type="button"
                    onClick={() => select(index)}
                    aria-expanded={open}
                    aria-controls={`pane-guide-${index}`}
                    className={styles.trigger}
                  >
                    <span aria-hidden="true" className={styles.index}>
                      0{index + 1}
                    </span>
                    <span className={styles.title}>{item.title}</span>
                    <ChevronDown size={16} aria-hidden="true" className={styles.chevron} />
                  </button>
                  <div id={`pane-guide-${index}`} className={styles.panel} aria-hidden={!open}>
                    <div>
                      {item.lines.map((line) => (
                        <p key={line}>{line}</p>
                      ))}
                    </div>
                  </div>
                </div>
              );
            })}
            <p className={styles.liveHint}>{copy.hint}</p>
          </div>

          <figure ref={preview} className={styles.figure} data-live-surface>
            <div className={styles.stage}>
              <span className={styles.chip} aria-live="polite">
                <b>0{active + 1}</b>
                {copy.items[active].title}
              </span>
              <span className={styles.liveBadge}>
                <i aria-hidden="true" />
                {copy.live}
              </span>

              <div className={styles.liveWrap}>
                <PaneDemoFrame lang={lang} className={styles.liveFrame} guide={selection} />
                {active === 6 ? (
                  <div className={styles.shortcutOverlay}>
                    <span className={styles.shortcutHead}>
                      {copy.items[6].title}
                      <Keyboard size={14} aria-hidden="true" />
                    </span>
                    {copy.shortcutKeys.map((row) => (
                      <span key={row.label} className={styles.shortcutRow}>
                        <span className={styles.keys}>
                          {row.keys.map((key, index) => (
                            <span key={key} className="inline-flex items-center gap-1">
                              {index > 0 ? <span className="text-muted-foreground text-xs">+</span> : null}
                              <kbd className={styles.kbd}>{key}</kbd>
                            </span>
                          ))}
                        </span>
                        <span>{row.label}</span>
                      </span>
                    ))}
                  </div>
                ) : null}
              </div>
            </div>
          </figure>
        </div>
      </div>
    </section>
  );
}
