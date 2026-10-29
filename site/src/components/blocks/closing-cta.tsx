import Link from "next/link";

import { AgentIcons } from "@/components/blocks/hero-headline";
import { AddToChromeCta } from "@/components/site/add-to-chrome-cta";
import { Button } from "@/components/ui/button";

export function ClosingCta() {
  return (
    <section aria-labelledby="closing-cta-heading" className="py-16 md:py-24 lg:py-32">
      <div className="container text-center">
        <p className="text-muted-foreground text-base font-medium tracking-widest">
          FROM INTENT TO IMPLEMENTATION
        </p>
        <h2
          id="closing-cta-heading"
          className="mx-auto mt-8 max-w-4xl text-5xl leading-[1.05] tracking-tight sm:text-6xl lg:text-[72px]"
        >
          Less explaining.
          <br />
          More making.
        </h2>
        <p className="text-muted-foreground mx-auto mt-8 max-w-xl text-base leading-relaxed">
          Make the change in your browser.
          <br />
          Give your agent the details to carry it into code.
        </p>
        <div className="mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <AddToChromeCta
            size="lg"
            className="h-14 max-sm:w-full max-sm:max-w-xs"
          />
          <Button
            asChild
            variant="outline"
            size="lg"
            className="h-14 max-sm:w-full max-sm:max-w-xs"
          >
            <Link href="/mcp">
              <AgentIcons size={16} />
              Connect your agent
            </Link>
          </Button>
        </div>
      </div>
    </section>
  );
}
