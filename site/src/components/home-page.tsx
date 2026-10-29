import { Background } from "@/components/background";
import { AgentWorkflow } from "@/components/blocks/agent-workflow";
import { BrowserHandoff } from "@/components/blocks/browser-handoff";
import { ClosingCta } from "@/components/blocks/closing-cta";
import { FAQ } from "@/components/blocks/faq";
import { Hero } from "@/components/blocks/hero";
import { PanelAnatomy } from "@/components/blocks/panel-anatomy";
import type { Lang } from "@/components/i18n";

export function HomePage({ lang }: { lang: Lang }) {
  return (
    <>
      <Background>
        <Hero lang={lang} />
      </Background>
      <AgentWorkflow lang={lang} />
      <PanelAnatomy lang={lang} />
      <FAQ />
      <BrowserHandoff />
      <Background variant="bottom">
        <ClosingCta />
      </Background>
    </>
  );
}
