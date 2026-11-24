"use client";

import { useEffect, useState } from "react";

import { usePathname } from "next/navigation";

import { ArrowUp } from "lucide-react";

import { langFromPath, messages } from "@/components/i18n";

export function BackToTop() {
  const pathname = usePathname();
  const [visible, setVisible] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const label = messages[langFromPath(pathname || "/")].footer.backToTop;

  useEffect(() => {
    const hero = document.querySelector("main section");
    const update = () => {
      const headerHeight =
        document.querySelector("header")?.getBoundingClientRect().height ?? 0;
      setVisible(
        window.scrollY > 0 &&
          (hero
            ? hero.getBoundingClientRect().bottom <= headerHeight
            : window.scrollY >= window.innerHeight),
      );
    };
    const frame = requestAnimationFrame(update);
    const observer = new ResizeObserver(update);
    if (hero) observer.observe(hero);
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, [pathname]);

  // The live demo is a real, clickable panel. A floating button would sit on
  // top of its lower-right cards on narrow screens, so step aside while any
  // interactive surface is on screen.
  useEffect(() => {
    const surfaces = document.querySelectorAll("[data-live-surface]");
    // This component lives in the root layout and survives client navigation,
    // so a route without a live surface must clear the flag it inherited.
    setBlocked(false);
    if (!surfaces.length) return;
    const seen = new Set<Element>();
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) seen.add(entry.target);
        else seen.delete(entry.target);
      }
      setBlocked(seen.size > 0);
    }, { threshold: 0.15 });
    surfaces.forEach((surface) => observer.observe(surface));
    return () => observer.disconnect();
  }, [pathname]);

  return (
    <button
      data-slot="button"
      type="button"
      aria-label={label}
      hidden={!visible || blocked}
      onClick={() => {
        window.scrollTo({
          top: 0,
          behavior: window.matchMedia("(prefers-reduced-motion: reduce)")
            .matches
            ? "instant"
            : "smooth",
        });
        document
          .querySelector<HTMLElement>("header a")
          ?.focus({ preventScroll: true });
      }}
      className="bg-background text-foreground hover:bg-accent focus-visible:ring-ring fixed right-4 bottom-4 z-40 flex size-12 items-center justify-center rounded-lg border shadow-md focus-visible:ring-2 focus-visible:outline-none md:right-8 md:bottom-8"
    >
      <span data-button-content>
        <ArrowUp className="size-4" aria-hidden="true" />
      </span>
    </button>
  );
}
