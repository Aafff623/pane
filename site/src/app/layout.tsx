import { Inter } from "next/font/google";

import type { Metadata, Viewport } from "next";

import { Footer } from "@/components/blocks/footer";
import { Navbar } from "@/components/blocks/navbar";
import { BackToTop } from "@/components/site/back-to-top";
import "@/styles/globals.css";

const themeScript = `(function(){try{var t=localStorage.getItem("pane-theme");if(t!=="dark"&&t!=="light"){t="light"}document.documentElement.classList.toggle("dark",t==="dark");document.documentElement.style.colorScheme=t}catch(e){}})();`;

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
  fallback: ["Arial", "Helvetica", "sans-serif"],
  adjustFontFallback: false,
  preload: true,
});

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#f9f9f9" },
    { media: "(prefers-color-scheme: dark)", color: "#08090a" },
  ],
};

export const metadata: Metadata = {
  metadataBase: new URL("https://pane.threetwoa.live"),
  title: {
    default: "Pane — 本地优先的 AI 用量工作台",
    template: "%s | Pane",
  },
  description:
    "Pane 把 AI 工具的额度、余额、重置时间、多账号与本地 CLI Token 花费汇集到一个随时可唤出的桌面面板。凭据保存在本机。",
  applicationName: "Pane",
  category: "Developer Tools",
  keywords: ["Pane", "AI usage", "AI quota", "token spend", "本地优先", "AI 用量", "Tauri", "Rust"],
  authors: [{ name: "Aafff623", url: "https://github.com/Aafff623/pane" }],
  creator: "Aafff623",
  openGraph: {
    type: "website",
    locale: "zh_CN",
    siteName: "Pane",
    title: "Pane — 本地优先的 AI 用量工作台",
    description:
      "Pane 把 AI 工具的额度、余额、重置时间、多账号与本地 CLI Token 花费汇集到一个随时可唤出的桌面面板。凭据保存在本机。",
    url: "https://pane.threetwoa.live/",
    images: [{ url: "/og-cover.png", width: 1200, height: 630, alt: "Pane" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "Pane — 本地优先的 AI 用量工作台",
    description:
      "Pane 把 AI 工具的额度、余额、重置时间、多账号与本地 CLI Token 花费汇集到一个随时可唤出的桌面面板。凭据保存在本机。",
    images: ["/og-cover.png"],
  },
  alternates: {
    canonical: "/",
    languages: { en: "/", "zh-CN": "/zh", "x-default": "/" },
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={inter.variable} data-scroll-behavior="smooth" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="antialiased">
        <a
          href="#main-content"
          className="bg-background text-foreground focus:ring-ring sr-only fixed top-4 left-4 z-[100] rounded-md px-4 py-2 font-medium focus:not-sr-only focus:ring-2 focus:outline-none"
        >
          Skip to content · 跳到主要内容
        </a>
        <Navbar />
        <main id="main-content" className="mx-auto w-full max-w-[1200px]">
          {children}
        </main>
        <Footer />
        <BackToTop />
      </body>
    </html>
  );
}
