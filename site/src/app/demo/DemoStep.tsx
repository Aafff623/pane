import Link from "next/link";

import styles from "./demo.module.scss";
import type { Step } from "./steps";

type Props = {
  step: Step;
  children?: React.ReactNode;
};

// Single step card: title, body paragraphs, "Try it" callout, and an
// optional slot below for the demo target component.
export function DemoStep({ step, children }: Props) {
  const isChild = !!step.parentId;
  return (
    <section
      id={step.id}
      data-step-id={step.id}
      className={`${styles.step} ${isChild ? styles.stepChild : ""}`}
    >
      <h2 className={styles.stepTitle}>{step.title}</h2>
      <div className={styles.stepBody}>
        {step.body.map((p, i) => (
          <p key={i}>{p}</p>
        ))}
      </div>
      {step.tryIt && (
        <div className={styles.tryIt} role="note">
          <span className={styles.tryItLabel}>Try it</span>
          <span className={styles.tryItBody}>{step.tryIt}</span>
        </div>
      )}
      {children && <div className={styles.stepTarget}>{children}</div>}
      {step.nextLink && (
        <div style={{ marginTop: '0.75rem' }}>
          <Link href={step.nextLink.href}>{step.nextLink.label}</Link>
        </div>
      )}
    </section>
  );
}
