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
      <Hero lang={lang} />
      <PanelAnatomy lang={lang} />
      <AgentWorkflow lang={lang} />
      <Trust lang={lang} />
      <FAQ lang={lang} />
      <DownloadSection lang={lang} />
      <Background variant="bottom" className="mx-auto mb-12 w-[calc(100%_-_2rem)] max-w-[1320px] sm:w-[calc(100%_-_3rem)]">
        <ClosingCta lang={lang} />
      </Background>
    </>
  );
}
