"use client";

import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

export function PaneDemoFrame({ lang, className, title }: { lang: "en" | "zh"; className?: string; title?: string }) {
  const [theme, setTheme] = useState<"light" | "dark">("light");

  useEffect(() => {
    const sync = () => setTheme(document.documentElement.classList.contains("dark") ? "dark" : "light");
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  return (
    <iframe
      src={`/demo/index.html?theme=${theme}&lang=${lang}`}
      title={title ?? "Pane interactive panel"}
      loading="lazy"
      className={cn(className)}
    />
  );
}
