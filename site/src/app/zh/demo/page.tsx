import type { Metadata } from "next";

import { DemoEmbed } from "@/components/blocks/demo-embed";
import { DocumentLang } from "@/components/document-lang";
import { messages } from "@/components/i18n";

const copy = messages.zh.demoPage;

export const metadata: Metadata = {
  title: "在线演示",
  description: copy.description,
  alternates: { canonical: "https://pane.threetwoa.live/zh/demo" },
};

export default function DemoPage() {
  return (
    <section className="py-16 md:py-24">
      <DocumentLang lang="zh" />
      <div className="container max-w-5xl">
        <div className="mx-auto max-w-[760px] text-center">
          <h1 className="text-3xl tracking-tight md:text-5xl">{copy.title}</h1>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] leading-relaxed">{copy.description}</p>
        </div>
        <div className="mt-12">
          <DemoEmbed lang="zh" />
        </div>
      </div>
    </section>
  );
}
