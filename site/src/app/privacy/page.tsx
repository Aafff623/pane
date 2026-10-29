"use client";

import Privacy from "./privacy.mdx";
import Reporting from "./reporting.mdx";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";

export default function PrivacyPage() {
  return (
    <>
      {/* Hero — yellow background slab */}
      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-5xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              Privacy
            </h1>
            <p className="text-muted-foreground mt-4 max-w-3xl text-base">
              Design Mode runs in your browser. Release builds send usage
              analytics without page content. Here is what else can leave your
              machine, when, and why.
            </p>
          </div>
        </section>
      </Background>

      {/* Middle — main privacy content */}
      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-5xl" />
        <article className="prose container mx-auto mt-12 max-w-3xl">
          <Privacy />
        </article>
      </section>

      {/* Bottom — reporting concerns in yellow slab */}
      <Background variant="bottom">
        <section className="py-20 lg:py-28">
          <DashedLine className="container max-w-5xl" />
          <article className="prose container mx-auto mt-12 max-w-3xl">
            <Reporting />
          </article>
        </section>
      </Background>
    </>
  );
}
