"use client";

import { useEffect } from "react";

// Delegated outbound / mailto click tracker. Dedicated CTAs in the
// navbar fire their own `cta_click` events; this catches everything
// else (inline GitHub / mailto / footer links) without forcing every
// link author to remember an onClick.
type GtagFn = (
  command: "event",
  action: string,
  params: Record<string, unknown>,
) => void;

export function LinkTracker() {
  useEffect(() => {
    const onClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      const anchor = target.closest("a") as HTMLAnchorElement | null;
      if (!anchor) return;
      const href = anchor.getAttribute("href") || "";
      if (!href) return;

      const w = window as unknown as { gtag?: GtagFn };
      if (!w.gtag) return;

      if (href.startsWith("mailto:") || href.startsWith("tel:")) {
        const [scheme, rest = ""] = href.split(":");
        w.gtag("event", "contact_click", { scheme, target: rest.split("?")[0] });
        return;
      }

      if (/^https?:\/\//i.test(href)) {
        try {
          const url = new URL(href);
          if (url.host !== window.location.host) {
            w.gtag("event", "outbound_click", {
              destination_host: url.host,
              destination_url: href,
              link_text: (anchor.textContent || "").trim().slice(0, 120),
            });
          }
        } catch {
          /* malformed URL — drop quietly */
        }
      }
    };

    document.addEventListener("click", onClick);
    return () => document.removeEventListener("click", onClick);
  }, []);

  return null;
}
