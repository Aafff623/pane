"use client";

import { useState } from "react";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { LangToggle } from "@/components/lang-toggle";
import { ThemeToggle } from "@/components/theme-toggle";
import { langFromPath, messages } from "@/components/i18n";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { NavigationMenu, NavigationMenuItem, NavigationMenuList } from "@/components/ui/navigation-menu";

const REPO_URL = "https://github.com/Aafff623/pane";

function GithubMark({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}

export const Navbar = () => {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const pathname = usePathname() || "/";
  const lang = langFromPath(pathname);
  const copy = messages[lang].nav;
  const base = lang === "zh" ? "/zh" : "";
  const navHref = (href: string) => (href.startsWith("#") ? `${base}${href}` : href);

  return (
    <header className="bg-background text-foreground sticky top-0 z-50 w-full border-b">
      <div className="mx-auto grid min-h-16 max-w-[1440px] grid-cols-[1fr_auto] items-center gap-4 px-6 py-2 xl:grid-cols-[1fr_auto_1fr]">
        <Link href={base || "/"} className="flex shrink-0 items-center gap-2">
          <Image src="/pane-logo.png" width={24} height={24} alt="" unoptimized className="size-6 rounded-md" />
          <span className="font-display text-foreground text-base font-semibold tracking-tight">Pane</span>
        </Link>

        <NavigationMenu className="max-xl:hidden">
          <NavigationMenuList className="gap-0">
            {copy.items.map((link) => (
              <NavigationMenuItem key={link.label}>
                <Link
                  href={navHref(link.href)}
                  className={cn(
                    "text-foreground hover:bg-accent rounded-lg px-4 py-2 text-base font-medium transition-colors",
                    pathname === link.href && "font-semibold",
                  )}
                >
                  {link.label}
                </Link>
              </NavigationMenuItem>
            ))}
          </NavigationMenuList>
        </NavigationMenu>

        <div className="flex items-center gap-1 justify-self-end">
          <Button asChild variant="ghost" size="icon-sm">
            <a href={REPO_URL} target="_blank" rel="noopener noreferrer" aria-label={copy.github}>
              <GithubMark size={16} />
            </a>
          </Button>
          <LangToggle />
          <ThemeToggle />
          <div className="max-xl:hidden">
            <Button asChild size="sm">
              <a href="/download/windows">{copy.download}</a>
            </Button>
          </div>

          <button
            data-slot="button"
            className="text-foreground relative flex size-8 xl:hidden"
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            aria-label={isMenuOpen ? "Close main menu" : "Open main menu"}
            aria-expanded={isMenuOpen}
            aria-controls="mobile-navigation"
          >
            <span data-button-content className="relative m-auto block w-4">
              <span aria-hidden="true" className={`absolute block h-0.5 w-full rounded-full bg-current transition duration-500 ease-in-out ${isMenuOpen ? "rotate-45" : "-translate-y-1.5"}`} />
              <span aria-hidden="true" className={`absolute block h-0.5 w-full rounded-full bg-current transition duration-500 ease-in-out ${isMenuOpen ? "opacity-0" : ""}`} />
              <span aria-hidden="true" className={`absolute block h-0.5 w-full rounded-full bg-current transition duration-500 ease-in-out ${isMenuOpen ? "-rotate-45" : "translate-y-1.5"}`} />
            </span>
          </button>
        </div>
      </div>

      <div
        id="mobile-navigation"
        aria-hidden={!isMenuOpen}
        inert={!isMenuOpen}
        className={cn(
          "bg-background text-foreground absolute inset-x-0 top-full flex max-h-[calc(100dvh-5rem)] flex-col overflow-y-auto border-b p-6 shadow-xl transition-all duration-300 ease-in-out motion-reduce:transition-none xl:hidden",
          isMenuOpen ? "visible translate-y-0 opacity-100" : "invisible -translate-y-4 opacity-0",
        )}
      >
        <nav className="divide-border flex flex-1 flex-col divide-y">
          {copy.items.map((link) => (
            <Link
              key={link.label}
              href={navHref(link.href)}
              className={cn("text-foreground hover:bg-accent rounded-lg px-4 py-2 text-base font-medium transition-colors first:mt-0", pathname === link.href && "font-semibold")}
              onClick={() => setIsMenuOpen(false)}
            >
              {link.label}
            </Link>
          ))}
          <div className="pt-4">
            <Button asChild className="w-full">
              <a href="/download/windows">{copy.download}</a>
            </Button>
          </div>
        </nav>
      </div>
    </header>
  );
};
