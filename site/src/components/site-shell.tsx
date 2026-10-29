import type { ReactNode } from "react";

import { Footer } from "@/components/blocks/footer";
import { Navbar } from "@/components/blocks/navbar";
import { dictionaries, type Lang } from "@/i18n";

export function SiteShell({ lang, children }: { lang: Lang; children: ReactNode }) {
  const dict = dictionaries[lang];
  return (
    <>
      <Navbar lang={lang} dict={dict} />
      <main id="main-content">{children}</main>
      <Footer lang={lang} dict={dict} />
    </>
  );
}
