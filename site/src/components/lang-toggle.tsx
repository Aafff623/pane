"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

export function LangToggle() {
  const pathname = usePathname() || "/";
  const isZh = pathname === "/zh" || pathname.startsWith("/zh/");
  const strip = (path: string) => path.replace(/^\/zh(?=\/|$)/, "") || "/";
  const target = isZh ? strip(pathname) : `/zh${pathname === "/" ? "" : pathname}`;

  return (
    <Link
      href={target}
      aria-label={isZh ? "Switch to English" : "切换为中文"}
      className="text-foreground hover:bg-accent inline-flex h-8 items-center rounded-lg px-2 text-xs font-semibold tracking-widest uppercase transition-colors"
    >
      {isZh ? "EN" : "中"}
    </Link>
  );
}
