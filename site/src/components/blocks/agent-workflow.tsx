import type { ReactNode } from "react";

import { ArrowRight, Code as CodeIcon, Terminal } from "@phosphor-icons/react/dist/ssr";

import { ProviderMark } from "@/components/provider-mark";
import type { Dictionary, Lang } from "@/i18n";
import styles from "./agent-workflow.module.css";

function WorkflowWindow({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="border-border bg-background w-full rounded-2xl border md:w-auto md:flex-1">
      <div className="border-border flex items-center gap-2 border-b px-6 py-4">
        {[0, 1, 2].map((dot) => (
          <span key={dot} aria-hidden="true" className="bg-muted-foreground/30 size-2 rounded-full" />
        ))}
        <span className="text-muted-foreground ml-3 text-sm">{title}</span>
      </div>
      <div className="px-8 py-10">{children}</div>
    </div>
  );
}

export function Workflow({ lang, dict }: { lang: Lang; dict: Dictionary }) {
  void lang;
  return (
    <section id="features" aria-labelledby="agent-workflow-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container flex flex-col items-center justify-start">
        <div className="mx-0 w-full max-w-[920px] text-center">
          <p className="text-muted-foreground text-base font-medium tracking-widest uppercase">{dict.workflow.eyebrow}</p>
          <h2 id="agent-workflow-heading" className="mt-6 w-full text-2xl tracking-tight md:text-4xl lg:text-5xl">
            {dict.workflow.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] text-base leading-relaxed">{dict.workflow.description}</p>
        </div>

        <div className="mx-auto mt-16 flex w-full max-w-5xl flex-col items-center md:mt-24 md:flex-row md:items-stretch">
          <WorkflowWindow title={dict.workflow.leftWindow}>
            <div className="flex items-center gap-2">
              {["claude", "codex", "cursor", "qoder"].map((name) => (
                <ProviderMark key={name} name={name} size={22} className="text-foreground" />
              ))}
            </div>
            <h3 className="mt-6 text-2xl font-medium tracking-tight">{dict.workflow.leftHeading}</h3>
            <p className="text-muted-foreground mt-3 text-base leading-relaxed">{dict.workflow.leftBody}</p>
            <div className="text-muted-foreground mt-8 flex items-center gap-2 text-sm">
              <Terminal aria-hidden="true" className="size-4" />
              {dict.workflow.leftFoot}
            </div>
          </WorkflowWindow>

          <div className="flex h-32 shrink-0 flex-col items-center justify-center md:h-auto md:w-40">
            <span className="text-muted-foreground text-base font-medium tracking-widest">{dict.workflow.bridge}</span>
            <div aria-hidden="true" className="bg-border relative mt-3 h-12 w-px md:h-px md:w-full">
              <ArrowRight className={`text-muted-foreground size-4 ${styles.arrow}`} />
            </div>
            <span className="text-muted-foreground mt-3 text-sm">{dict.workflow.bridgeNote}</span>
          </div>

          <WorkflowWindow title={dict.workflow.rightWindow}>
            <CodeIcon aria-hidden="true" className="size-10 stroke-1" />
            <h3 className="mt-6 text-2xl font-medium tracking-tight">{dict.workflow.rightHeading}</h3>
            <p className="text-muted-foreground mt-3 text-base leading-relaxed">{dict.workflow.rightBody}</p>
            <div className="text-muted-foreground mt-8 flex items-center gap-2 text-sm">
              <span className="bg-primary inline-block size-2 rounded-full" aria-hidden="true" />
              {dict.workflow.rightFoot}
            </div>
          </WorkflowWindow>
        </div>
      </div>
    </section>
  );
}
