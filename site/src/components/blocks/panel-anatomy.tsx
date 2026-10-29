"use client";

import { useState } from "react";

import { Bell, Keyboard, LockKeyhole, Palette } from "lucide-react";

import { ChevronDown } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";
import styles from "./panel-stage.module.css";

const IMAGES = ["overview", "header", "toolbar", "overview", "spend"];
const WIDE = new Set([1, 2]);
const FOCUS: Record<number, number> = { 0: 0, 3: -10, 4: -2 };
const ROW_ICONS = [Keyboard, LockKeyhole, Palette, Bell];

function Shot({ name, wide, focus }: { name: string; wide?: boolean; focus?: number }) {
  if (wide) {
    return (
      <>
        <img src={`/img/anatomy/${name}.png`} alt="" className={`${styles.shot} ${styles.shotWide} block dark:hidden`} />
        <img src={`/img/anatomy/${name}-dark.png`} alt="" className={`${styles.shot} ${styles.shotWide} hidden dark:block`} />
      </>
    );
  }
  const style = { transform: `translateY(${focus ?? 0}%)` };
  return (
    <div className={styles.window}>
      <img src={`/img/anatomy/${name}.png`} alt="" className={`${styles.windowImg} block dark:hidden`} style={style} />
      <img src={`/img/anatomy/${name}-dark.png`} alt="" className={`${styles.windowImg} hidden dark:block`} style={style} />
    </div>
  );
}

function SettingsCard({ title, kicker, rows }: { title: string; kicker: string; rows: readonly { label: string; value: string }[] }) {
  return (
    <div className={styles.card} role="img" aria-label={title}>
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>
          {title}
          <span>{kicker}</span>
        </span>
        <i className="bg-primary inline-block size-2 rounded-full" aria-hidden="true" />
      </div>
      <div className={styles.cardBody}>
        {rows.map((row, index) => {
          const Icon = ROW_ICONS[index] ?? Keyboard;
          return (
            <div key={row.label} className={styles.row}>
              <span className={styles.rowLabel}>
                <Icon size={14} aria-hidden="true" />
                {row.label}
              </span>
              <span className={styles.rowValue}>{row.value}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ShortcutsCard({ title, kicker, keys }: { title: string; kicker: string; keys: readonly { keys: readonly string[]; label: string }[] }) {
  return (
    <div className={styles.card} role="img" aria-label={title}>
      <div className={styles.cardHead}>
        <span className={styles.cardTitle}>
          {title}
          <span>{kicker}</span>
        </span>
        <Keyboard size={15} aria-hidden="true" />
      </div>
      <div className={styles.cardBody}>
        {keys.map((row) => (
          <div key={row.label} className={styles.keyRow}>
            <span className={styles.keys}>
              {row.keys.map((key, index) => (
                <span key={key} className="inline-flex items-center gap-1">
                  {index > 0 ? <span className="text-muted-foreground text-xs">+</span> : null}
                  <kbd className={styles.kbd}>{key}</kbd>
                </span>
              ))}
            </span>
            <span>{row.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function PanelAnatomy({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].panel;
  const [active, setActive] = useState(0);

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
                  <button
                    type="button"
                    onClick={() => setActive(index)}
                    aria-expanded={open}
                    className={styles.trigger}
                  >
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
          </div>

          <figure className={styles.figure}>
            <div className={styles.stage}>
              <span className={styles.chip}>
                <b>0{active + 1}</b>
                {copy.items[active].title}
              </span>
              <div key={active} className={styles.body}>
                {active < IMAGES.length ? (
                  <Shot name={IMAGES[active]} wide={WIDE.has(active)} focus={FOCUS[active]} />
                ) : active === 5 ? (
                  <SettingsCard title={copy.items[5].title} kicker="PANE" rows={copy.settingsRows} />
                ) : (
                  <ShortcutsCard title={copy.items[6].title} kicker="SYSTEM" keys={copy.shortcutKeys} />
                )}
              </div>
            </div>
          </figure>
        </div>
      </div>
    </section>
  );
}
