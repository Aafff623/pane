import Link from "next/link";

import {
  Briefcase,
  Building2,
  Code2,
  FileEdit,
  Lightbulb,
  Megaphone,
  Palette,
  Ruler,
  Sparkles,
  Users,
  Wand2,
} from "lucide-react";

import { DashedLine } from "@/components/dashed-line";

const personas = [
  {
    icon: Palette,
    title: "Designers",
    blurb:
      "Design directly on the live product — not in a mockup file that drifts from the code.",
    href: "/use-cases/redesign-any-website",
  },
  {
    icon: Ruler,
    title: "Design engineers",
    blurb:
      "Own the loop from sketch to PR without context-switching between tools.",
    href: "/use-cases/tailwind-component-tuning",
  },
  {
    icon: Code2,
    title: "Frontend developers",
    blurb:
      "Iterate on UI visually, then have your AI agent commit the change.",
    href: "/use-cases/visual-editing-with-cursor",
  },
  {
    icon: Wand2,
    title: "Vibe coders & AI-coding-agent users",
    blurb:
      "Describe a change in English, see it on the real page, refine visually — Claude Code, Cursor, Windsurf, Cline, all over MCP.",
    href: "/use-cases/vibe-coding-with-claude-code",
  },
  {
    icon: Sparkles,
    title: "QA & UI testers",
    blurb:
      "Annotate broken layout, contrast bugs, copy errors on a staging URL — export the structured diff to developers with full context.",
    href: "/use-cases/ui-testing-export-to-developers",
  },
  {
    icon: Megaphone,
    title: "Product managers",
    blurb:
      "File visual bug reports that point at the exact pixel, not \"the button is wrong.\"",
    href: "/use-cases/bug-report-with-visual-diff",
  },
  {
    icon: FileEdit,
    title: "Content & marketing teams",
    blurb:
      "Fix microcopy live on the page, export the diff, no Figma round-trip.",
    href: "/use-cases/copy-edits-without-a-pr",
  },
  {
    icon: Lightbulb,
    title: "Indie hackers & solo founders",
    blurb:
      "Iterate on your own landing page with an AI agent in the loop.",
    href: "/use-cases/landing-page-iteration-for-indie-hackers",
  },
  {
    icon: Briefcase,
    title: "Agencies",
    blurb:
      "Make tweaks on the client's staging URL during review; hand engineering a precise spec.",
    href: "/use-cases/client-handoff-from-agency-to-engineering",
  },
  {
    icon: Building2,
    title: "Design-system maintainers",
    blurb:
      "Walk a deployed app, log every token drift, ship the audit report.",
    href: "/use-cases/design-system-audit",
  },
];

export function PersonaBlock() {
  return (
    <section id="who-is-this-for" className="py-20 lg:py-28">
      <div className="container max-w-6xl">
        <DashedLine className="text-muted-foreground mb-16" />
        <div className="mb-12 max-w-3xl">
          <h2 className="text-2xl tracking-tight md:text-4xl lg:text-5xl">
            One design tool for every maker
          </h2>
          <p className="text-muted-foreground mt-4 text-base leading-snug">
            Designers, developers, QA testers, PMs, content people, indie
            hackers, agencies, and vibe coders all share the same problem —
            getting from a visual idea to working code without losing
            intent. Design Mode is the single surface that fits every step
            of that loop.
          </p>
        </div>

        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
          {personas.map((p) => {
            const Icon = p.icon;
            return (
              <Link
                key={p.title}
                href={p.href}
                className="group bg-card rounded-2xl border p-6 shadow-sm transition-shadow hover:shadow-md"
              >
                <Icon className="text-foreground group-hover:text-primary mb-4 size-6 transition-colors" />
                <h3 className="font-display text-base font-semibold">
                  {p.title}
                </h3>
                <p className="text-muted-foreground mt-2 text-base leading-relaxed">
                  {p.blurb}
                </p>
                <span className="text-foreground/80 mt-4 inline-flex items-center gap-2 text-base font-medium underline underline-offset-8 opacity-0 transition-opacity group-hover:opacity-100">
                  See the workflow →
                </span>
              </Link>
            );
          })}
        </div>

        <div className="mt-10 flex items-center gap-2">
          <Users className="text-muted-foreground size-4" />
          <Link
            href="/use-cases"
            className="text-foreground/80 text-base underline underline-offset-8 hover:text-foreground"
          >
            See every use case →
          </Link>
        </div>
      </div>
    </section>
  );
}
