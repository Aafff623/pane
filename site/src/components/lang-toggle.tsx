"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

export function LangToggle() {
  const pathname = usePathname() || "/";
  const isZh = pathname === "/zh" || pathname.startsWith("/zh/");
  // Only the homepage has a translated route so far; keep the switch on a
  // working target instead of linking to a /zh page that does not exist yet.
  const target = isZh ? "/" : "/zh";

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
