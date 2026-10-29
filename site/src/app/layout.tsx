import type { Metadata } from "next";

import { SITE_FACTS } from "@/content/facts";
import { zh } from "@/i18n";
import "@/styles/globals.css";

const themeScript = `(function(){try{var t=localStorage.getItem('pane-site-theme');if(t!=='dark'&&t!=='light'){t=window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'}document.documentElement.classList.toggle('dark',t==='dark');document.documentElement.style.colorScheme=t}catch(e){}})();`;

export const metadata: Metadata = {
  metadataBase: new URL(SITE_FACTS.domain),
  title: {
    default: zh.meta.title,
    template: `%s | Pane`,
  },
  description: zh.meta.description,
  applicationName: "Pane",
  category: "Developer Tools",
  keywords: [
    "Pane",
    "AI usage dashboard",
    "AI quota tracker",
    "token spend",
    "Claude Code usage",
    "Codex usage",
    "Cursor usage",
    "本地优先",
    "AI 用量",
  ],
  authors: [{ name: "Aafff623", url: SITE_FACTS.repo }],
  creator: "Aafff623",
  openGraph: {
    type: "website",
    locale: "zh_CN",
    siteName: "Pane",
    title: zh.meta.title,
    description: zh.meta.description,
    url: SITE_FACTS.domain,
    images: [{ url: "/og-cover.png", width: 1200, height: 630, alt: zh.meta.ogAlt }],
  },
  twitter: {
    card: "summary_large_image",
    title: zh.meta.title,
    description: zh.meta.description,
    images: ["/og-cover.png"],
  },
  icons: { icon: "/favicon.png" },
  alternates: {
    canonical: "/",
    languages: {
      "zh-CN": "/",
      en: "/en/",
      "x-default": "/",
    },
  },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="zh-CN" suppressHydrationWarning data-scroll-behavior="smooth">
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="min-h-dvh antialiased">
        <a
          href="#main-content"
          className="bg-background text-foreground focus:ring-ring sr-only rounded-lg px-4 py-2 focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-[100] focus:ring-2"
        >
          跳到主要内容
        </a>
        {children}
      </body>
    </html>
  );
}
