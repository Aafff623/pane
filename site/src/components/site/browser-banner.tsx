"use client";

import Image from "next/image";

import {
  useInstallTarget,
  type InstallTarget,
} from "@/hooks/use-install-target";
import {
  CHROME_WEB_STORE_URL,
  FIREFOX_AMO_URL,
  SAFARI_RELEASE_URL,
} from "@/lib/install-links";
import { withNavRef } from "@/lib/nav-ref";

// Browser-aware "also available" strip under the navbar. It names the two
// browsers the visitor is NOT using: Firefox + Safari on Chrome/Chromium/
// unknown (also the server-rendered default, so hydration matches), Chrome +
// Safari on Firefox, Chrome + Firefox on Safari. Only the browser names link.

const BROWSERS: Record<
  InstallTarget,
  { name: string; href: string; icon: string }
> = {
  chrome: {
    name: "Chrome",
    href: withNavRef(CHROME_WEB_STORE_URL),
    icon: "/chrome.svg",
  },
  firefox: {
    name: "Firefox",
    href: withNavRef(FIREFOX_AMO_URL),
    icon: "/firefox.svg",
  },
  safari: {
    name: "Safari",
    href: SAFARI_RELEASE_URL,
    icon: "/safari.svg",
  },
};

const ORDER: InstallTarget[] = ["chrome", "firefox", "safari"];

const LINK_CLASS =
  "focus-visible:ring-ring inline-flex items-center gap-1.5 align-bottom whitespace-nowrap font-medium underline underline-offset-4 focus-visible:ring-2 focus-visible:outline-none";

function BrowserLink({
  href,
  icon,
  name,
}: {
  href: string;
  icon: string;
  name: string;
}) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className={LINK_CLASS}
    >
      <Image
        src={icon}
        width={16}
        height={16}
        alt=""
        unoptimized
        className="shrink-0"
      />
      {name}
    </a>
  );
}

export function BrowserBanner() {
  const current = useInstallTarget();
  const [first, second] = ORDER.filter((b) => b !== current).map(
    (b) => BROWSERS[b],
  );
  return (
    <div className="w-full bg-[color-mix(in_srgb,var(--primary)_12%,white)]">
      <p className="text-foreground mx-auto min-h-8 px-4 py-2 text-center text-base leading-snug break-words">
        Design Mode is also available for{" "}
        <BrowserLink href={first.href} icon={first.icon} name={first.name} />{" "}
        and{" "}
        <BrowserLink href={second.href} icon={second.icon} name={second.name} />
        .
      </p>
    </div>
  );
}
