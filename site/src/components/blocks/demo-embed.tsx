import { messages, type Lang } from "@/components/i18n";
import { PaneDemoFrame } from "@/components/pane-demo-frame";

export function DemoEmbed({ lang }: { lang: Lang }) {
  const copy = messages[lang].demoPage;
  const src = `/pane-demo/index.html?theme=light&lang=${lang}`;

  return (
    <div className="border-border bg-card overflow-hidden rounded-2xl border shadow-lg">
      <div className="border-border flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <span className="text-primary inline-flex items-center gap-2 text-xs font-medium tracking-widest uppercase">
          <i className="bg-primary inline-block size-1.5 rounded-full" aria-hidden="true" />
          {copy.note}
        </span>
        <a
          href={src}
          target="_blank"
          rel="noopener noreferrer"
          className="text-foreground hover:bg-accent rounded-lg border border-border px-3 py-1.5 text-sm font-medium transition-colors"
        >
          {copy.open}
        </a>
      </div>
      <div className="bg-muted/40 flex justify-center px-4 py-8 sm:py-12">
        <div className="border-border bg-background w-full max-w-[460px] overflow-hidden rounded-xl border shadow-xl">
          <PaneDemoFrame lang={lang} title="Pane demo" className="h-[660px] w-full" />
        </div>
      </div>
    </div>
  );
}
