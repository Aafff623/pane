import Image from "next/image";
import Link from "next/link";

import { ChevronRight } from "lucide-react";

import { DashedLine } from "../dashed-line";

import { Card, CardContent } from "@/components/ui/card";

const items = [
  {
    title: "Inspect a rendered webpage",
    image: "/features/inspect-card.png",
    href: "/features",
  },
  {
    title: "Edit type, colour, layout, structure",
    image: "/features/edit-card.png",
    href: "/demo",
  },
  {
    title: "Hand the exact changes to your agent",
    image: "/features/ship-card.png",
    href: "/mcp",
  },
];

export const Features = () => {
  return (
    <section id="features" className="py-32">
      <div className="container">
        {/* Top dashed line with text */}
        <div className="relative flex items-center justify-center">
          <DashedLine className="text-muted-foreground" />
          <span className="border-border bg-card text-muted-foreground absolute rounded-lg border px-4 py-2 text-base font-medium tracking-wide max-md:hidden">
            DESIGN ON THE LIVE PAGE.
          </span>
        </div>

        {/* Content — vertical stack of title + body */}
        <div className="mx-auto mt-10 flex max-w-4xl flex-col gap-4 lg:mt-24">
          <h2 className="text-2xl tracking-tight md:text-4xl lg:text-5xl">
            From visual edit to reviewed source diff
          </h2>
          <p className="text-muted-foreground max-w-2xl leading-snug">
            Edit the rendered interface with visual controls. Design Mode
            records selectors, properties, old values, new values, text edits,
            DOM changes and comments. Use{" "}
            <Link
              href="/docs/changes-tab"
              className="underline underline-offset-4"
            >
              Copy as prompt
            </Link>{" "}
            for Markdown, or let a connected MCP client read it. The client
            needs separate repository access to implement the changes. Add
            intent and verification with the{" "}
            <Link
              href="/blog/turn-visual-edits-into-precise-ai-prompts"
              className="underline underline-offset-4"
            >
              precise-prompts guide
            </Link>{" "}
            before reviewing the source diff.
          </p>
        </div>

        {/* Features Card */}
        <Card className="mt-8 rounded-3xl md:mt-12 lg:mt-20">
          <CardContent className="flex p-0 max-md:flex-col">
            {items.map((item, i) => (
              <div key={i} className="flex flex-1 max-md:flex-col">
                <div className="flex-1 p-4 pe-0! md:p-6">
                  <div className="relative aspect-[1.28/1] overflow-hidden">
                    <Image
                      src={item.image}
                      alt={`${item.title} interface`}
                      fill
                      sizes="(min-width: 1280px) 384px, (min-width: 768px) 33vw, calc(100vw - 2rem)"
                      className="object-cover object-left-top ps-4 pt-2"
                    />
                    <div className="from-background absolute inset-0 z-10 bg-linear-to-t via-transparent to-transparent" />
                  </div>

                  <Link
                    href={item.href}
                    className="group flex items-center justify-between gap-4 pe-4 pt-4 md:pe-6 md:pt-6"
                  >
                    <h3 className="font-display max-w-60 text-2xl leading-tight font-bold tracking-tight">
                      {item.title}
                    </h3>
                    <div className="group-hover:border-primary group-hover:bg-primary/10 rounded-lg border p-2 transition-colors">
                      <ChevronRight className="size-6 transition-transform group-hover:translate-x-1 lg:size-8" />
                    </div>
                  </Link>
                </div>
                {i < items.length - 1 && (
                  <div className="relative hidden md:block">
                    <DashedLine orientation="vertical" />
                  </div>
                )}
                {i < items.length - 1 && (
                  <div className="relative block md:hidden">
                    <DashedLine orientation="horizontal" />
                  </div>
                )}
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </section>
  );
};
