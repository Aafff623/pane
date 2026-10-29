export const dynamic = "force-static";

import type { MetadataRoute } from "next";

const BASE = "https://pane.threetwoa.live";

const PAIRS = [
  { en: "/", zh: "/zh", priority: 1 },
  { en: "/demo", zh: "/zh/demo", priority: 0.7 },
  { en: "/privacy", zh: "/zh/privacy", priority: 0.3 },
  { en: "/terms", zh: "/zh/terms", priority: 0.3 },
];

export default function sitemap(): MetadataRoute.Sitemap {
  return PAIRS.flatMap(({ en, zh, priority }) => {
    const languages = { en: `${BASE}${en}`, "zh-CN": `${BASE}${zh}` };
    return [
      { url: `${BASE}${en}`, lastModified: new Date("2026-10-29"), changeFrequency: "monthly" as const, priority, alternates: { languages } },
      { url: `${BASE}${zh}`, lastModified: new Date("2026-10-29"), changeFrequency: "monthly" as const, priority: Math.max(0.2, priority - 0.1), alternates: { languages } },
    ];
  });
}
