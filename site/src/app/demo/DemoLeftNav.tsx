"use client";

import { useEffect, useState } from "react";

import styles from "./demo.module.scss";
import { STEPS, type Step } from "./steps";

// Sticky left nav. Lists every step (with Design sub-anchors nested under
// their parent). Tracks the most-visible section via IntersectionObserver
// and highlights the matching link.
export function DemoLeftNav() {
  const [activeId, setActiveId] = useState<string>(STEPS[0]?.id ?? "");

  useEffect(() => {
    // Observe each section and pick the one closest to the top of the
    // viewport with the most intersection ratio.
    const sections = STEPS.map((s) =>
      document.getElementById(s.id)
    ).filter((el): el is HTMLElement => !!el);

    if (sections.length === 0) return;

    let visibleEntries: IntersectionObserverEntry[] = [];

    const observer = new IntersectionObserver(
      (entries) => {
        // Replace any entries we already had for these targets, then pick
        // the one with the highest intersection ratio that's also near
        // the top of the viewport.
        const map = new Map(visibleEntries.map((e) => [e.target, e]));
        for (const e of entries) map.set(e.target, e);
        visibleEntries = Array.from(map.values()).filter((e) => e.isIntersecting);
        if (visibleEntries.length === 0) return;
        // Closest to top — pick the entry whose top is smallest positive.
        const topMost = visibleEntries.reduce((best, e) =>
          e.boundingClientRect.top < best.boundingClientRect.top ? e : best
        );
        const id = (topMost.target as HTMLElement).id;
        if (id) setActiveId(id);
      },
      {
        // Trigger when the section's top crosses the upper third of the
        // viewport — feels right when you scroll-down step by step.
        rootMargin: "-20% 0px -60% 0px",
        threshold: [0, 0.25, 0.5, 0.75, 1],
      }
    );

    sections.forEach((s) => observer.observe(s));
    return () => observer.disconnect();
  }, []);

  function handleClick(e: React.MouseEvent<HTMLAnchorElement>, id: string) {
    e.preventDefault();
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "start" });
      // Update active immediately so the click feels responsive instead of
      // waiting for the IntersectionObserver to settle.
      setActiveId(id);
      // Update the URL hash for shareable links.
      history.replaceState(null, "", `#${id}`);
    }
  }

  // Build a flat list of nav entries, but indent children under their parent.
  const topLevel = STEPS.filter((s) => !s.parentId);

  return (
    <nav className={styles.sideNav} aria-label="Demo sections">
      <ol className={styles.sideNavList}>
        {topLevel.map((step, idx) => {
          const children = STEPS.filter((s) => s.parentId === step.id);
          return (
            <li key={step.id} className={styles.sideNavItem}>
              <a
                href={`#${step.id}`}
                className={styles.sideNavLink}
                data-active={activeId === step.id ? "true" : "false"}
                onClick={(e) => handleClick(e, step.id)}
              >
                <span className={styles.sideNavNumber}>{idx + 1}</span>
                <span>{step.title}</span>
              </a>
              {children.length > 0 && (
                <ol className={styles.sideNavSubList}>
                  {children.map((child, ci) => (
                    <li key={child.id}>
                      <a
                        href={`#${child.id}`}
                        className={styles.sideNavSubLink}
                        data-active={activeId === child.id ? "true" : "false"}
                        onClick={(e) => handleClick(e, child.id)}
                      >
                        <span className={styles.sideNavSubLetter}>
                          {String.fromCharCode("a".charCodeAt(0) + ci)}
                        </span>
                        <span>{child.title}</span>
                      </a>
                    </li>
                  ))}
                </ol>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

// Re-exporting the Step type alongside is helpful for the consuming page.
export type { Step };
