import type { ReactNode } from "react";

import Image from "next/image";

import { ArrowRight, Code2, MousePointer2 } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";
import styles from "./agent-workflow.module.css";

const HARNESS = [
  { logo: "claude", colour: "#D97757", name: "Claude" },
  { logo: "openai", colour: "#10A37F", name: "Codex" },
  { logo: "gemini-color", name: "Gemini" },
  { logo: "cursor", colour: "#26241E", name: "Cursor" },
  { logo: "windsurf", colour: "#07847D", name: "Windsurf" },
  { logo: "perplexity", colour: "#20808D", name: "Perplexity" },
  { logo: "antigravity-color", name: "Antigravity" },
];

function HarnessMarks() {
  return (
    <div className="flex flex-wrap items-center gap-2" aria-hidden="true">
      {HARNESS.map((tool) => (
        <span key={tool.logo} className="border-border bg-card flex size-9 items-center justify-center rounded-lg border">
          <span
            className="block size-4"
            style={
              tool.colour
                ? {
                    backgroundColor: tool.colour,
                    maskImage: `url(/logos/icons/${tool.logo}.svg)`,
                    maskSize: "contain",
                    maskRepeat: "no-repeat",
                    maskPosition: "center",
                    WebkitMaskImage: `url(/logos/icons/${tool.logo}.svg)`,
                    WebkitMaskSize: "contain",
                    WebkitMaskRepeat: "no-repeat",
                    WebkitMaskPosition: "center",
                  }
                : {
                    backgroundImage: `url(/logos/icons/${tool.logo}.svg)`,
                    backgroundSize: "contain",
                    backgroundRepeat: "no-repeat",
                    backgroundPosition: "center",
                  }
            }
          />
        </span>
      ))}
    </div>
  );
}

function WorkflowWindow({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="border-border bg-background w-full rounded-2xl border md:w-auto md:flex-1">
      <div className="border-border flex items-center gap-2 border-b px-6 py-4">
        {[0, 1, 2].map((dot) => (
          <span key={dot} aria-hidden="true" className="bg-muted-foreground/30 size-2 rounded-full" />
        ))}
        <span className="text-muted-foreground ml-3 text-base">{title}</span>
      </div>
      <div className="px-8 py-10">{children}</div>
    </div>
  );
}

export function AgentWorkflow({ lang = "en" }: { lang?: Lang }) {
  const copy = messages[lang].workflow;

  return (
    <section aria-labelledby="agent-workflow-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container flex flex-col items-center justify-start">
        <div className="mx-0 w-full max-w-[980px] text-center">
          <p className="text-muted-foreground text-base font-medium tracking-widest uppercase">{copy.eyebrow}</p>
          <h2 id="agent-workflow-heading" className="mt-6 w-full text-2xl tracking-tight md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] text-base leading-relaxed">{copy.description}</p>
        </div>

        <div className="mx-auto mt-16 flex w-full max-w-5xl flex-col items-center md:mt-24 md:flex-row md:items-stretch">
          <WorkflowWindow title={copy.leftTitle}>
            <Image src="/pane-logo.png" width={40} height={40} alt="" unoptimized className="rounded-lg" />
            <h3 className="mt-6 text-2xl font-medium tracking-tight">{copy.leftTitle}</h3>
            <p className="text-muted-foreground mt-3 text-base leading-relaxed">{copy.leftBody}</p>
            <div className="text-muted-foreground mt-8 flex items-center gap-2 text-base">
              <MousePointer2 aria-hidden="true" className="size-4" />
              {copy.leftFoot}
            </div>
          </WorkflowWindow>

          <div className="flex h-32 shrink-0 flex-col items-center justify-center md:h-auto md:w-40">
            <span className="text-muted-foreground text-base font-medium tracking-widest">{copy.bridge}</span>
            <div aria-hidden="true" className="bg-border relative mt-3 h-12 w-px md:h-px md:w-full">
              <ArrowRight className={`text-muted-foreground size-4 ${styles.arrow}`} />
            </div>
            <span className="text-muted-foreground mt-3 text-base">{copy.bridgeNote}</span>
          </div>

          <WorkflowWindow title={copy.rightTitle}>
            <HarnessMarks />
            <h3 className="mt-6 text-2xl font-medium tracking-tight">{copy.rightTitle}</h3>
            <p className="text-muted-foreground mt-3 text-base leading-relaxed">{copy.rightBody}</p>
            <div className="text-muted-foreground mt-8 flex items-center gap-2 text-base">
              <Code2 aria-hidden="true" className="size-4" />
              <span className="text-base">{copy.rightFoot}</span>
            </div>
          </WorkflowWindow>
        </div>
      </div>
    </section>
  );
}
