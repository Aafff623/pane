"use client";

import { useEffect, useRef, useState } from "react";

import type { DemoGuide } from "@/content/demo-guide";
import { cn } from "@/lib/utils";

export function PaneDemoFrame({ lang, className, title, guide }: { lang: "en" | "zh"; className?: string; title?: string; guide?: { step: DemoGuide } }) {
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const frame = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    if (guide) frame.current?.contentWindow?.postMessage({ type: "pane-demo-guide", step: guide.step }, location.origin);
  }, [guide]);

  useEffect(() => {
    const sync = () => setTheme(document.documentElement.classList.contains("dark") ? "dark" : "light");
    sync();
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  return (
    <iframe
      ref={frame}
      onLoad={() => {
        if (guide) frame.current?.contentWindow?.postMessage({ type: "pane-demo-guide", step: guide.step }, location.origin);
      }}
      src={`/pane-demo/index.html?theme=${theme}&lang=${lang}`}
      title={title ?? "Pane interactive panel"}
      loading="lazy"
      className={cn(className)}
    />
  );
}
