import { Background } from "@/components/background";
import { AgentWorkflow } from "@/components/blocks/agent-workflow";
import { ClosingCta } from "@/components/blocks/closing-cta";
import { DownloadSection } from "@/components/blocks/download";
import { FAQ } from "@/components/blocks/faq";
import { Hero } from "@/components/blocks/hero";
import { PanelAnatomy } from "@/components/blocks/panel-anatomy";
import { Trust } from "@/components/blocks/trust";
import type { Lang } from "@/components/i18n";

export function HomePage({ lang }: { lang: Lang }) {
  return (
    <>
      <Background>
        <Hero lang={lang} />
      </Background>
      <AgentWorkflow lang={lang} />
      <PanelAnatomy lang={lang} />
      <Trust lang={lang} />
      <FAQ lang={lang} />
      <DownloadSection lang={lang} />
      <Background variant="bottom">
        <ClosingCta lang={lang} />
      </Background>
    </>
  );
}
