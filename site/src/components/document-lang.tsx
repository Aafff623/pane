"use client";

import { useEffect } from "react";

export function DocumentLang({ lang }: { lang: "zh" }) {
  useEffect(() => {
    const previous = document.documentElement.lang;
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    return () => {
      document.documentElement.lang = previous;
    };
  }, [lang]);
  return null;
}
