"use client";

import { useState } from "react";

import { ChevronDown, Keyboard } from "lucide-react";

import { PaneDemoFrame } from "@/components/pane-demo-frame";
import { messages, type Lang } from "@/components/i18n";
import styles from "./panel-stage.module.css";

type Rect = { left: number; top: number; width: number; height: number };

const FOCUS: Record<number, Rect> = {
  0: { left: 6, top: 50, width: 88, height: 47 },
  1: { left: 3, top: 1.5, width: 94, height: 11 },
  2: { left: 3, top: 46, width: 94, height: 12 },
  3: { left: 3, top: 58, width: 47, height: 24 },
  4: { left: 3, top: 7, width: 94, height: 43 },
  5: { left: 82, top: 1.5, width: 15, height: 9 },
};

function LivePanel({ lang }: { lang: Lang }) {
  return <PaneDemoFrame lang={lang} className={styles.liveFrame} />;
}

export function PanelAnatomy({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].panel;
  const [active, setActive] = useState(0);
  const focus = FOCUS[active];

  return (
    <section id="panel-anatomy" aria-labelledby="panel-anatomy-title" className="py-16 md:py-24 lg:py-32">
      <div className="container">
        <h2 id="panel-anatomy-title" className="text-center text-3xl tracking-tight text-balance sm:text-4xl md:text-5xl lg:text-6xl">
          {copy.title}
        </h2>

        <div className={styles.walkthrough}>
          <div className={styles.list}>
            {copy.items.map((item, index) => {
              const open = index === active;
              return (
                <div key={item.title} className={styles.item} data-open={open}>
                  <button type="button" onClick={() => setActive(index)} aria-expanded={open} className={styles.trigger}>
                    <span aria-hidden="true" className={styles.index}>
                      0{index + 1}
                    </span>
                    <span className={styles.title}>{item.title}</span>
                    <ChevronDown size={16} aria-hidden="true" className={styles.chevron} />
                  </button>
                  <div className={styles.panel}>
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

          <figure className={styles.figure}>
            <div className={styles.stage}>
              <span className={styles.chip}>
                <b>0{active + 1}</b>
                {copy.items[active].title}
              </span>
              <span className={styles.liveBadge}>
                <i aria-hidden="true" />
                {copy.live}
              </span>

              <div className={styles.liveWrap}>
                <LivePanel lang={lang} />
                {focus ? (
                  <span
                    aria-hidden="true"
                    className={styles.focusRing}
                    style={{ left: `${focus.left}%`, top: `${focus.top}%`, width: `${focus.width}%`, height: `${focus.height}%` }}
                  />
                ) : null}
                {active === 6 ? (
                  <div className={styles.shortcutOverlay} role="img" aria-label={copy.items[6].title}>
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
