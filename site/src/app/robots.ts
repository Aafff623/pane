export const dynamic = "force-static";

import type { MetadataRoute } from "next";

import { SITE_FACTS } from "@/content/facts";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      { userAgent: "*", allow: "/", disallow: ["/demo/"] },
      { userAgent: "GPTBot", allow: "/" },
      { userAgent: "ClaudeBot", allow: "/" },
      { userAgent: "PerplexityBot", allow: "/" },
      { userAgent: "Googlebot", allow: "/" },
      { userAgent: "Bingbot", allow: "/" },
    ],
    sitemap: `${SITE_FACTS.domain}/sitemap.xml`,
    host: SITE_FACTS.domain,
  };
}
