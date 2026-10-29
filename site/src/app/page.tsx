import type { Metadata } from "next";

import { HomePage } from "@/components/home-page";
import { zh } from "@/i18n";

export const metadata: Metadata = {
  title: { absolute: zh.meta.title },
  description: zh.meta.description,
  alternates: { canonical: "/", languages: { "zh-CN": "/", en: "/en/", "x-default": "/" } },
  openGraph: { locale: "zh_CN", url: "/", title: zh.meta.title, description: zh.meta.description },
};

export default function Page() {
  return <HomePage lang="zh" />;
}
