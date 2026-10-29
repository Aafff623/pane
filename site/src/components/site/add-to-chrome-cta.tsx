"use client";

import Image from "next/image";

import { Button } from "@/components/ui/button";
import { useInstallTarget } from "@/hooks/use-install-target";
import {
  CHROME_WEB_STORE_URL,
  FIREFOX_AMO_URL,
  SAFARI_RELEASE_URL,
} from "@/lib/install-links";
import { withNavRef } from "@/lib/nav-ref";
import { cn } from "@/lib/utils";

// Browser-aware install CTA. Chrome Web Store on Chromium and unknown
// browsers, addons.mozilla.org (AMO) on Firefox, and the latest GitHub
// release on Safari (manual install); label + icon follow suit. Named
// AddToChromeCta for its (many) existing call sites.

const STORES = {
  chrome: {
    name: "Chrome",
    label: "Add to Chrome",
    href: withNavRef(CHROME_WEB_STORE_URL),
    icon: "/chrome.svg",
    event: "add_to_chrome",
  },
  firefox: {
    name: "Firefox",
    label: "Add to Firefox",
    href: withNavRef(FIREFOX_AMO_URL),
    icon: "/firefox.svg",
    event: "add_to_firefox",
  },
  safari: {
    name: "Safari",
    label: "Download latest",
    href: SAFARI_RELEASE_URL,
    icon: "/safari.svg",
    event: "download_safari",
  },
} as const;

type GtagFn = (
  command: "event",
  action: string,
  params: Record<string, unknown>,
) => void;

function trackCtaClick(cta: string) {
  const w = window as unknown as { gtag?: GtagFn };
  if (w.gtag) w.gtag("event", "cta_click", { cta });
}

export function AddToChromeCta({
  size = "default",
  label,
  className,
  iconSize = 16,
}: {
  size?: "default" | "sm" | "lg";
  label?: string;
  className?: string;
  iconSize?: number;
}) {
  const store = STORES[useInstallTarget()];
  return (
    <Button asChild size={size} className={cn("gap-2", className)}>
      <a
        href={store.href}
        target="_blank"
        rel="noopener noreferrer"
        onClick={() => trackCtaClick(store.event)}
      >
        <Image
          src={store.icon}
          width={iconSize}
          height={iconSize}
          alt=""
          aria-hidden="true"
          className="shrink-0"
        />
        {label ?? store.label}
      </a>
    </Button>
  );
}

// Small icon-only link to the OTHER browser's store, so users know Design
// Mode is available elsewhere. Navbar-only: sits between the GitHub icon and
// the primary CTA. On Firefox it points at Chrome/CWS; otherwise (Chrome,
// Safari, unknown) at Firefox/AMO.
export function OtherStoreLink({ className }: { className?: string }) {
  const target = useInstallTarget();
  const other = target === "firefox" ? STORES.chrome : STORES.firefox;
  return (
    <Button asChild variant="ghost" size="icon-sm" className={className}>
      <a
        href={other.href}
        target="_blank"
        rel="noopener noreferrer"
        onClick={() => trackCtaClick(`also_${other.event}`)}
        aria-label={`Also available on ${other.name}`}
        title={`Also available on ${other.name}`}
      >
        <Image
          src={other.icon}
          width={16}
          height={16}
          alt=""
          aria-hidden="true"
          className="shrink-0"
        />
      </a>
    </Button>
  );
}
