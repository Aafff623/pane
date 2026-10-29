import Link from "next/link";

import { ArrowRight } from "@phosphor-icons/react/dist/ssr";

import { DownloadButton } from "@/components/download-button";
import { ProviderMark } from "@/components/provider-mark";
import { Button } from "@/components/ui/button";
import type { Dictionary, Lang } from "@/i18n";
import { SITE_FACTS } from "@/content/facts";
import styles from "./hero-headline.module.css";

const CLUSTER = ["claude", "codex", "cursor", "copilot", "deepseek", "minimax", "qoder"];

export function Hero({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  const base = lang === "zh" ? "" : "/en";
  return (
    <section className="pt-16 pb-12 md:pt-24 md:pb-16 lg:pt-28">
      <div className="container max-w-[1200px] text-center max-sm:px-4">
        <div className={styles.cluster} aria-hidden="true">
          {CLUSTER.map((name, index) => (
            <span key={name} className={styles.mark} style={{ animationDelay: `${index * 1.7}s` }}>
              <ProviderMark name={name} size={18} />
            </span>
          ))}
        </div>

        <p className="text-muted-foreground text-sm font-medium tracking-[0.18em] uppercase">{dict.hero.eyebrow}</p>

        <h1 className={`${styles.headline} macro text-foreground mt-6`}>
          <span className={styles.line}>{dict.hero.titleLead}</span>
          <span className={`${styles.line} ${styles.focusLine}`}>{dict.hero.titleFocus}</span>
        </h1>

        <p className="text-muted-foreground mx-auto mt-8 max-w-[760px] text-base leading-relaxed">{dict.hero.description}</p>

        <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <DownloadButton
            labels={{ windows: dict.hero.download, macos: dict.download.macos }}
            size="lg"
            className="h-14 w-full sm:w-auto"
          />
          <Button asChild variant="outline" size="lg" className="h-14 w-full sm:w-auto">
            <Link href={`${base}#demo`}>
              {dict.hero.demo}
              <ArrowRight size={16} aria-hidden="true" />
            </Link>
          </Button>
        </div>

        <dl className="border-border mx-auto mt-12 grid max-w-[720px] grid-cols-3 border-y py-6">
          <div>
            <dt className="text-muted-foreground text-xs tracking-wider uppercase">{dict.hero.stats.providers}</dt>
            <dd className="text-foreground mt-1 text-2xl font-semibold tabular-nums">{SITE_FACTS.providers}</dd>
          </div>
          <div className="border-border border-x">
            <dt className="text-muted-foreground text-xs tracking-wider uppercase">{dict.hero.stats.accounts}</dt>
            <dd className="text-foreground mt-1 text-2xl font-semibold tabular-nums">0</dd>
          </div>
          <div>
            <dt className="text-muted-foreground text-xs tracking-wider uppercase">{dict.hero.stats.license}</dt>
            <dd className="text-foreground mt-1 text-2xl font-semibold">MIT</dd>
          </div>
        </dl>

        <figure className="mx-auto mt-12 w-full max-w-[1040px] sm:mt-16">
          <div className="border-border relative aspect-[8/5] overflow-hidden rounded-2xl border bg-[radial-gradient(120%_90%_at_50%_0%,color-mix(in_srgb,var(--primary)_16%,transparent),transparent_60%)] shadow-lg">
            <div aria-hidden="true" className="absolute inset-0 opacity-[0.18] [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:44px_44px]" />
            <div
              role="img"
              aria-label={dict.hero.imageAlt}
              className="border-border absolute top-8 left-1/2 h-[calc(100%-2rem)] aspect-[439/640] -translate-x-1/2 overflow-hidden rounded-t-xl border shadow-2xl"
            >
              <div
                className="block h-full w-full bg-[length:100%_auto] bg-top bg-no-repeat dark:hidden"
                style={{ backgroundImage: "url('/img/pane-panel-light.png')" }}
              />
              <div
                className="hidden h-full w-full bg-[length:100%_auto] bg-top bg-no-repeat dark:block"
                style={{ backgroundImage: "url('/img/pane-panel-dark.png')" }}
              />
            </div>
          </div>
          <figcaption className="text-muted-foreground mt-3 text-sm">{dict.hero.caption}</figcaption>
        </figure>
      </div>
    </section>
  );
}
