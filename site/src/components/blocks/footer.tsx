import Link from "next/link";
import Image from "next/image";

import type { Dictionary, Lang } from "@/i18n";

export function Footer({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  const groups = [
    { title: dict.footer.product, links: dict.footer.productLinks },
    { title: dict.footer.resources, links: dict.footer.resourceLinks },
    { title: dict.footer.project, links: dict.footer.projectLinks },
  ];

  return (
    <footer className="border-t py-16 md:pt-24 md:pb-8">
      <div className="container">
        <div className="grid gap-12 md:grid-cols-[1.4fr_repeat(3,1fr)]">
          <div>
            <Link href={lang === "zh" ? "/" : "/en/"} className="flex items-center gap-2">
              <Image src="/brand-logo.png" width={24} height={24} alt="" unoptimized className="size-6" />
              <span className="font-display text-base font-semibold tracking-tight">Pane</span>
            </Link>
            <p className="text-muted-foreground mt-4 max-w-xs text-sm leading-relaxed">{dict.meta.description}</p>
          </div>
          {groups.map((group) => (
            <div key={group.title}>
              <p className="text-foreground text-sm font-semibold">{group.title}</p>
              <ul className="mt-4 space-y-2">
                {group.links.map((link) => {
                  const external = link.href.startsWith("http");
                  return (
                    <li key={link.label}>
                      {external ? (
                        <a
                          href={link.href}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-muted-foreground hover:text-foreground text-sm transition-colors"
                        >
                          {link.label}
                        </a>
                      ) : (
                        <Link href={link.href} className="text-muted-foreground hover:text-foreground text-sm transition-colors">
                          {link.label}
                        </Link>
                      )}
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>
        <div className="border-border text-muted-foreground mt-16 flex flex-col gap-3 border-t pt-6 text-xs leading-relaxed md:flex-row md:items-center md:justify-between">
          <p className="max-w-3xl">{dict.footer.disclaimer}</p>
          <p className="shrink-0">{dict.footer.copyright}</p>
        </div>
      </div>
    </footer>
  );
}
