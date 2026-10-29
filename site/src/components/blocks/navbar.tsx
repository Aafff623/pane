"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { GithubLogo, List, X } from "@phosphor-icons/react";

import { DownloadButton } from "@/components/download-button";
import { ThemeToggle } from "@/components/theme-toggle";
import type { Dictionary, Lang } from "@/i18n";
import { cn } from "@/lib/utils";

export function Navbar({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  const [open, setOpen] = useState(false);
  const pathname = usePathname() || "/";
  const base = lang === "zh" ? "" : "/en";
  const neutral = pathname.replace(/^\/en(?=\/|$)/, "") || "/";
  const languageHref = lang === "zh" ? `/en${neutral === "/" ? "/" : neutral}` : neutral;
  const links = [
    { label: dict.nav.features, href: `${base}/#features` },
    { label: dict.nav.demo, href: `${base}/#demo` },
    { label: dict.nav.faq, href: `${base}/#faq` },
  ];

  return (
    <header className="bg-background/90 supports-[backdrop-filter]:bg-background/70 sticky top-0 z-50 w-full border-b backdrop-blur">
      <div className="mx-auto grid min-h-16 max-w-[1440px] grid-cols-[1fr_auto] items-center gap-4 px-6 py-2 xl:grid-cols-[1fr_auto_1fr]">
        <Link href={base || "/"} className="flex shrink-0 items-center gap-2">
          <Image src="/brand-logo.png" width={24} height={24} alt="" unoptimized className="size-6" />
          <span className="font-display text-foreground text-base font-semibold tracking-tight">Pane</span>
        </Link>

        <nav className="max-xl:hidden" aria-label={dict.nav.primary}>
          <ul className="flex items-center gap-0">
            {links.map((link) => (
              <li key={link.label}>
                <Link
                  href={link.href}
                  className="text-foreground hover:bg-accent rounded-lg px-4 py-2 text-base font-medium transition-colors"
                >
                  {link.label}
                </Link>
              </li>
            ))}
          </ul>
        </nav>

        <div className="flex items-center gap-1 justify-self-end">
          <Link
            href={`https://github.com/Aafff623/pane`}
            target="_blank"
            rel="noopener noreferrer"
            aria-label="GitHub"
            className="text-foreground hover:bg-accent inline-flex size-9 items-center justify-center rounded-lg transition-colors"
          >
            <GithubLogo size={18} aria-hidden="true" />
          </Link>
          <ThemeToggle label={dict.nav.theme} />
          <Link
            href={languageHref}
            aria-label={dict.nav.language}
            className="text-foreground hover:bg-accent inline-flex h-9 items-center rounded-lg px-3 text-xs font-semibold tracking-widest uppercase transition-colors"
          >
            {lang === "zh" ? "EN" : "中"}
          </Link>
          <div className="max-xl:hidden">
            <DownloadButton labels={{ windows: dict.hero.download, macos: dict.download.macosCta }} size="sm" />
          </div>
          <button
            type="button"
            onClick={() => setOpen((value) => !value)}
            aria-label={open ? dict.nav.closeMenu : dict.nav.menu}
            aria-expanded={open}
            aria-controls="mobile-navigation"
            className="text-foreground hover:bg-accent relative inline-flex size-9 items-center justify-center rounded-lg xl:hidden"
          >
            {open ? <X size={18} aria-hidden="true" /> : <List size={18} aria-hidden="true" />}
          </button>
        </div>
      </div>

      <div
        id="mobile-navigation"
        aria-hidden={!open}
        inert={!open}
        className={cn(
          "bg-background text-foreground absolute inset-x-0 top-full flex max-h-[calc(100dvh-5rem)] flex-col overflow-y-auto border-b p-6 shadow-xl transition-all duration-300 ease-in-out motion-reduce:transition-none xl:hidden",
          open ? "visible translate-y-0 opacity-100" : "invisible -translate-y-4 opacity-0",
        )}
      >
        <nav className="divide-border flex flex-1 flex-col divide-y" aria-label="Mobile">
          {links.map((link) => (
            <Link
              key={link.label}
              href={link.href}
              onClick={() => setOpen(false)}
              className="text-foreground hover:bg-accent rounded-lg px-4 py-3 text-base font-medium transition-colors"
            >
              {link.label}
            </Link>
          ))}
          <div className="pt-4">
            <DownloadButton
              labels={{ windows: dict.hero.download, macos: dict.download.macosCta }}
              className="w-full"
            />
          </div>
        </nav>
      </div>
    </header>
  );
}
