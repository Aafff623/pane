import type { Metadata } from "next";

import { HomePage } from "@/components/home-page";
import { en } from "@/i18n";
import { SITE_FACTS } from "@/content/facts";

export const metadata: Metadata = {
  title: { absolute: en.meta.title },
  description: en.meta.description,
  alternates: { canonical: "/en/", languages: { "zh-CN": "/", en: "/en/", "x-default": "/" } },
  openGraph: {
    locale: "en_US",
    url: `${SITE_FACTS.domain}/en/`,
    title: en.meta.title,
    description: en.meta.description,
  },
};

export default function Page() {
  return <HomePage lang="en" />;
}
