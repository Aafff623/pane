import type { ReactNode } from "react";

import { Footer } from "@/components/blocks/footer";
import { Navbar } from "@/components/blocks/navbar";
import { dictionaries, type Lang } from "@/i18n";

export function SiteShell({ lang, children }: { lang: Lang; children: ReactNode }) {
  const dict = dictionaries[lang];
  return (
    <>
      <a
        href="#main-content"
        className="bg-background text-foreground focus:ring-ring sr-only rounded-lg px-4 py-2 focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-[100] focus:ring-2"
      >
        {dict.nav.skip}
      </a>
      <Navbar lang={lang} dict={dict} />
      <main id="main-content">{children}</main>
      <Footer lang={lang} dict={dict} />
    </>
  );
}
