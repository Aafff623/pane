"use client";

import { useEffect, useState } from "react";

import { SITE_FACTS } from "@/content/facts";
import { cn } from "@/lib/utils";

type Props = {
  label: string;
  macLabel?: string;
  className?: string;
  variant?: "primary" | "outline";
};

type Target = { label: string; href: string; external: boolean };

function detectOS() {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac OS X|Macintosh/i.test(ua)) return "macos";
  if (/Linux|X11/i.test(ua)) return "linux";
  return "other";
}

export function SmartDownload({ label, macLabel, className, variant = "primary" }: Props) {
  const [target, setTarget] = useState<Target>({ label, href: "/download/windows", external: false });

  useEffect(() => {
    let alive = true;
    (async () => {
      const os = detectOS();
      if (os === "macos") {
        let href = SITE_FACTS.releases;
        try {
          const res = await fetch("/api/latest", { headers: { Accept: "application/json" } });
          if (res.ok) {
            const data = await res.json();
            const dmgs = (data?.assets ?? []).filter((a: { name?: string }) => /\.dmg$/i.test(a.name ?? ""));
            const pick = dmgs.find((a: { name: string }) => /arm64|aarch64/i.test(a.name)) ?? dmgs[0];
            if (pick?.url) href = pick.url;
          }
        } catch {}
        if (alive) setTarget({ label: macLabel ?? label, href, external: true });
        return;
      }
      if (os === "linux") {
        if (alive) setTarget({ label: "Linux", href: SITE_FACTS.releases, external: true });
      }
    })();
    return () => {
      alive = false;
    };
  }, [label, macLabel]);

  return (
    <a
      href={target.href}
      {...(target.external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
      data-slot="button"
      className={cn(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg px-4 py-2 text-base leading-6 font-semibold transition-colors [&_svg]:size-4",
        variant === "primary"
          ? "bg-primary text-primary-foreground shadow-sm hover:brightness-[0.97]"
          : "border-border bg-card text-foreground shadow-xs hover:bg-accent",
        className,
      )}
    >
      <span data-button-content className="inline-flex items-center justify-center [gap:inherit]">
        {target.label}
      </span>
    </a>
  );
}
