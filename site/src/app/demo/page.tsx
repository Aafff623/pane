import type { Metadata } from "next";

import { DemoEmbed } from "@/components/blocks/demo-embed";
import { messages } from "@/components/i18n";

const copy = messages.en.demoPage;

export const metadata: Metadata = {
  title: "Live demo",
  description: copy.description,
  alternates: { canonical: "https://pane.threetwoa.live/demo/" },
};

export default function DemoPage() {
  return (
    <section className="py-16 md:py-24">
      <div className="container max-w-5xl">
        <div className="mx-auto max-w-[760px] text-center">
          <h1 className="text-3xl tracking-tight md:text-5xl">{copy.title}</h1>
          <p className="text-muted-foreground mx-auto mt-6 max-w-[640px] leading-relaxed">{copy.description}</p>
        </div>
        <div className="mt-12">
          <DemoEmbed lang="en" />
        </div>
      </div>
    </section>
  );
}
