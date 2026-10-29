import { ClosingCta } from "@/components/blocks/closing-cta";
import { DemoSection } from "@/components/blocks/demo-section";
import { DownloadSection } from "@/components/blocks/download-section";
import { FAQ } from "@/components/blocks/faq";
import { Hero } from "@/components/blocks/hero";
import { PanelAnatomy } from "@/components/blocks/panel-anatomy";
import { Workflow } from "@/components/blocks/agent-workflow";
import { SiteShell } from "@/components/site-shell";
import { dictionaries, type Lang } from "@/i18n";

export function HomePage({ lang }: { lang: Lang }) {
  const dict = dictionaries[lang];
  return (
    <SiteShell lang={lang}>
      <Hero lang={lang} dict={dict} />
      <Workflow lang={lang} dict={dict} />
      <PanelAnatomy dict={dict} />
      <DemoSection lang={lang} dict={dict} />
      <FAQ dict={dict} />
      <DownloadSection dict={dict} />
      <ClosingCta lang={lang} dict={dict} />
    </SiteShell>
  );
}
