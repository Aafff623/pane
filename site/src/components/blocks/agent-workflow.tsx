import type { ReactNode } from "react";

import Image from "next/image";

import { ArrowRight, Code2, MousePointer2 } from "lucide-react";

import { messages, type Lang } from "@/components/i18n";
import { SITE_FACTS } from "@/content/facts";
import styles from "./agent-workflow.module.css";

const HARNESS = [
  ["claude", "Claude Code"], ["codex", "Codex"], ["cursor", "Cursor"], ["windsurf", "Windsurf"],
  ["copilot", "GitHub Copilot"], ["antigravity", "Antigravity"], ["qodercn", "Qoder CN"],
  ["kiro", "Kiro"], ["zed", "Zed"], ["opencode", "OpenCode"], ["clinepass", "ClinePass"],
  ["codebuddy", "CodeBuddy"], ["workbuddy", "WorkBuddy"], ["trae", "Trae"],
  ["kimi", "Kimi"], ["zai", "Z.ai"], ["minimax", "MiniMax"], ["deepseek", "DeepSeek"],
  ["openrouter", "OpenRouter"], ["grok", "Grok"], ["amp", "Amp"], ["mistral", "Mistral"],
  ["qwen", "Qwen"], ["hermes", "Hermes"], ["bibigpt", "BibiGPT"],
  ["perplexity", "Perplexity"], ["firecrawl", "Firecrawl"], ["tavily", "Tavily"],
] as const;

// The wall of 28 logos read as clutter, so the card shows a settled cluster and
// lets an orbiting ring carry the "and many more" idea, anchored by an ellipsis
// tile. All 28 still ship — the rest are reachable from the providers doc.
const CLUSTER = HARNESS.slice(0, 5);
const ORBIT = HARNESS.slice(5, 15);

function HarnessMarks() {
  return (
    <div className={styles.marksStage}>
      <ul className={styles.orbit} aria-hidden="true">
        {ORBIT.map(([logo], index) => {
          const angle = (360 / ORBIT.length) * index;
          return (
            <li
              key={logo}
              className={styles.orbitItem}
              style={{ transform: `rotate(${angle}deg) translateY(var(--orbit-radius)) rotate(${-angle}deg)` }}
            >
              <span className={styles.orbitIcon}>
                <Image src={`/logos/providers/${logo}.svg`} alt="" width={18} height={18} className="size-[18px] object-contain" />
              </span>
            </li>
          );
        })}
      </ul>
      <ul className={styles.providerMarks}>
        {CLUSTER.map(([logo, name]) => (
          <li key={logo} title={name} className={styles.clusterMark}>
            <Image src={`/logos/providers/${logo}.svg`} alt={name} width={19} height={19} className="size-[55%] object-contain" />
          </li>
        ))}
        <li className={styles.moreMark} aria-hidden="true">
          <span className={styles.moreDots}>···</span>
        </li>
      </ul>
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
    <section id="features" aria-labelledby="agent-workflow-heading" className="py-14 md:py-20">
      <div className="container flex flex-col items-center justify-start">
        <div className="mx-0 w-full max-w-[980px] text-center">
          <p className="text-muted-foreground text-sm font-medium tracking-wider">{copy.eyebrow}</p>
          <h2 id="agent-workflow-heading" className="mt-4 w-full text-3xl tracking-tight text-balance md:text-4xl lg:text-5xl">
            {copy.title}
          </h2>
          <p className="text-muted-foreground mx-auto mt-4 max-w-[640px] text-base leading-relaxed">{copy.description}</p>
        </div>

        <div className="mx-auto mt-9 flex w-full max-w-5xl flex-col items-center md:mt-12 md:flex-row md:items-stretch">
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
            <a href={`${SITE_FACTS.repo}/blob/main/docs/providers.md`} className="text-primary mt-6 inline-flex items-center gap-2 text-sm font-medium underline-offset-4 hover:underline">
              <Code2 aria-hidden="true" className="size-4" />{copy.rightFoot}
            </a>
          </WorkflowWindow>
        </div>
      </div>
    </section>
  );
}
