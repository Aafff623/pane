export const dynamic = "force-static";

import type { MetadataRoute } from "next";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [{ userAgent: "*", allow: "/", disallow: ["/pane-demo/index.html"] }],
    sitemap: "https://pane.threetwoa.live/sitemap.xml",
    host: "https://pane.threetwoa.live",
  };
}
