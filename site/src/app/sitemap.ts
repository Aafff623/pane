export const dynamic = "force-static";

import type { MetadataRoute } from "next";

import { SITE_FACTS } from "@/content/facts";

const PAIRS = [
  { zh: "/", en: "/en/", priority: 1 },
  { zh: "/privacy/", en: "/en/privacy/", priority: 0.3 },
  { zh: "/terms/", en: "/en/terms/", priority: 0.3 },
];

export default function sitemap(): MetadataRoute.Sitemap {
  return PAIRS.flatMap(({ zh, en, priority }) => {
    const languages = { "zh-CN": `${SITE_FACTS.domain}${zh}`, en: `${SITE_FACTS.domain}${en}` };
    return [
      { url: `${SITE_FACTS.domain}${zh}`, lastModified: new Date("2026-10-29"), changeFrequency: "monthly" as const, priority, alternates: { languages } },
      { url: `${SITE_FACTS.domain}${en}`, lastModified: new Date("2026-10-29"), changeFrequency: "monthly" as const, priority: Math.max(0.2, priority - 0.1), alternates: { languages } },
    ];
  });
}
