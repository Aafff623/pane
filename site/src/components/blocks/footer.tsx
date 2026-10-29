"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { langFromPath, messages } from "@/components/i18n";

export function Footer() {
  const pathname = usePathname() || "/";
  const lang = langFromPath(pathname);
  const copy = messages[lang].footer;
  const groups = [
    { title: copy.product, links: copy.productLinks },
    { title: copy.resources, links: copy.resourceLinks },
    { title: copy.project, links: copy.projectLinks },
  ];

  return (
    <footer className="border-t py-16 md:pt-24 md:pb-8">
      <div className="container">
        <div className="grid gap-12 md:grid-cols-[1.4fr_repeat(3,1fr)]">
          <div>
            <Link href={lang === "zh" ? "/zh" : "/"} className="flex items-center gap-2">
              <Image src="/pane-logo.png" width={24} height={24} alt="" unoptimized className="size-6 rounded-md" />
              <span className="font-display text-base font-semibold tracking-tight">Pane</span>
            </Link>
            <p className="text-muted-foreground mt-4 max-w-xs text-sm leading-relaxed">
              {lang === "zh"
                ? "本地优先的 AI 用量工作台。额度、余额、重置时间与本地 CLI 花费，一个面板。"
                : "A local-first AI usage workspace for quotas, balances, reset times and local CLI spend."}
            </p>
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
          <p className="max-w-3xl">{copy.disclaimer}</p>
          <p className="shrink-0">{copy.copyright}</p>
        </div>
      </div>
    </footer>
  );
}
