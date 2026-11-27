import Link from "next/link";

import { HeroHeadline } from "@/components/blocks/hero-headline";
import { HeroFilm } from "@/components/home-motion";
import { SITE_FACTS } from "@/content/facts";
import { messages, type Lang } from "@/components/i18n";
import { SmartDownload } from "@/components/smart-download";
import { Button } from "@/components/ui/button";

import styles from "./hero.module.css";

export function Hero({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].hero;
  const demoHref = lang === "zh" ? "/zh/demo/" : "/demo/";
  // "0" as a proof point read like "zero accounts" on a landing page. These
  // four are the facts a visitor actually cares about, and "No sign-up" is the
  // honest version of why Pane needs no account: everything it reads is already
  // on the machine.
  const proof = [String(SITE_FACTS.providers), copy.noSignup, "MIT", `v${SITE_FACTS.version}`];

  return (
    <section className={styles.hero} aria-labelledby="hero-title">
      <div className={styles.layout}>
        <div className={styles.copy}>
          <HeroHeadline lead={copy.lead} finish={copy.finish} />
          <p className={styles.description}>{copy.description}</p>
          <div className={styles.actions}>
            <SmartDownload label={copy.primary} macLabel={copy.macPrimary} className={styles.button} />
            <Button variant="outline" className={styles.button} asChild>
              <Link href={demoHref}>{copy.secondary}</Link>
            </Button>
          </div>
          <dl className={styles.proof}>
            {proof.map((value, index) => (
              <div className={styles.fact} key={copy.proof[index] ?? value}>
                <dt>{copy.proof[index]}</dt>
                <dd className="metric-value">{value}</dd>
              </div>
            ))}
          </dl>
        </div>
        <figure className={styles.showcase}>
          <div className={styles.stage}>
            <div className={styles.filmWrap}>
              <HeroFilm lang={lang} label={copy.previewAlt} className={styles.film} buttonClassName={styles.filmToggle} />
            </div>
          </div>
          <figcaption>{copy.previewCaption}</figcaption>
        </figure>
      </div>
    </section>
  );
}
