import styles from "./hero-headline.module.css";

export function HeroHeadline({ lead, finish }: { lead: string; finish: string }) {
  return (
    <h1 id="hero-title" className={styles.headline}>
      <span>{lead}</span>{" "}
      <span className={styles.accent}>{finish}</span>
    </h1>
  );
}
