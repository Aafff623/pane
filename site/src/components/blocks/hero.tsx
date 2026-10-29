import Link from "next/link";

import { HeroHeadline } from "@/components/blocks/hero-headline";
import { SITE_FACTS } from "@/content/facts";
import { messages, type Lang } from "@/components/i18n";
import { SmartDownload } from "@/components/smart-download";
import { Button } from "@/components/ui/button";

export const Hero = ({ lang = "en" }: { lang?: Lang }) => {
  const copy = messages[lang].hero;
  const demoHref = lang === "zh" ? "/zh/demo" : "/demo";
  const proof = [
    { value: String(SITE_FACTS.providers), label: copy.proof[0] },
    { value: "0", label: copy.proof[1] },
    { value: "MIT", label: copy.proof[2] },
    { value: `v${SITE_FACTS.version}`, label: copy.proof[3] },
  ];

  return (
    <section className="py-16 md:py-24 lg:py-32">
      <div className="container max-w-[1200px] text-center max-sm:px-4">
        <HeroHeadline lead={copy.lead} before={copy.before} after={copy.after} />

        <p className="text-muted-foreground mx-auto mt-8 max-w-[848px] text-base">{copy.description}</p>

        <div className="mt-12 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <SmartDownload
            label={copy.primary}
            macLabel={copy.macPrimary}
            className="h-14 w-full sm:w-auto text-base"
          />
          <Button variant="outline" size="lg" className="h-14 w-full sm:w-auto text-base" asChild>
            <Link href={demoHref}>{copy.secondary}</Link>
          </Button>
        </div>

        <dl className="border-border mx-auto mt-12 grid max-w-[860px] grid-cols-2 gap-y-6 border-y py-6 sm:grid-cols-4">
          {proof.map((item) => (
            <div key={item.label} className="flex flex-col items-center">
              <dt className="text-muted-foreground order-2 mt-1 text-xs tracking-wider uppercase">{item.label}</dt>
              <dd className="order-1 text-2xl font-semibold tabular-nums">{item.value}</dd>
            </div>
          ))}
        </dl>

        <figure className="mx-auto mt-12 w-full max-w-[1040px] sm:mt-16">
          <div className="border-border relative aspect-[8/5] overflow-hidden rounded-2xl border bg-[radial-gradient(120%_90%_at_50%_0%,color-mix(in_srgb,var(--primary)_14%,transparent),transparent_60%)] shadow-lg">
            <div
              aria-hidden="true"
              className="absolute inset-0 opacity-[0.16] [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:44px_44px]"
            />
            <div
              role="img"
              aria-label={lang === "zh" ? "Pane 面板：额度、花费与多账号" : "Pane panel: quotas, spend and accounts"}
              className="border-border absolute top-8 left-1/2 hidden h-[calc(100%-2rem)] aspect-[439/640] -translate-x-1/2 rounded-t-xl border bg-[url('/img/pane-panel-dark.png')] bg-[length:100%_auto] bg-top bg-no-repeat shadow-2xl dark:block"
            />
            <div
              role="img"
              aria-label={lang === "zh" ? "Pane 面板：额度、花费与多账号" : "Pane panel: quotas, spend and accounts"}
              className="border-border absolute top-8 left-1/2 block h-[calc(100%-2rem)] aspect-[439/640] -translate-x-1/2 rounded-t-xl border bg-[url('/img/pane-panel-light.png')] bg-[length:100%_auto] bg-top bg-no-repeat shadow-2xl dark:hidden"
            />
            <span className="bg-background/80 text-muted-foreground border-border absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full border px-3 py-1 text-xs backdrop-blur">
              {lang === "zh" ? "示例数据 · 真实面板" : "Sample data · real panel"}
            </span>
          </div>
          <figcaption className="text-muted-foreground mt-3 text-sm">
            {lang === "zh"
              ? "额度、花费、重置时间和多账号，都在一个面板里。"
              : "Quotas, spend, reset times and accounts in one panel."}
          </figcaption>
        </figure>
      </div>
    </section>
  );
};
