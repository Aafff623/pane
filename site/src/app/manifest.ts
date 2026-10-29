export const dynamic = "force-static";

import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Pane",
    short_name: "Pane",
    description: "Local-first AI usage dashboard for quotas, balances, reset times and local CLI token spend.",
    start_url: "/",
    display: "standalone",
    background_color: "#f9f9f9",
    theme_color: "#087d6d",
    icons: [
      { src: "/icon.png", sizes: "512x512", type: "image/png" },
      { src: "/icon.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
  };
}
