import Link from "next/link";

import { Button } from "@/components/ui/button";
import { withNavRef } from "@/lib/nav-ref";

const REPO_URL = withNavRef("https://github.com/SandeepBaskaran/design-mode");

// Exported separately so the page can wrap it in <Background variant="bottom">.
export function WhyThisExists() {
  return (
    <section className="container max-w-5xl">
      <TextSection
        title="Why this exists"
        paragraphs={[
          "Screenshots and prose are useful for broad direction, but they often lose selectors, computed values and the relationship between the intended change and the rendered page.",
          "Design Mode lets you make the intended change on the rendered page, records a structured specification, and hands it to a coding agent as Markdown or through MCP. The agent still has to find the source, implement the change and return a diff for review.",
          "Built in the open under MIT. Read the source, file an issue, or open a PR.",
        ]}
        ctaButton={{
          href: REPO_URL,
          text: "View on GitHub",
          external: true,
        }}
      />
    </section>
  );
}

interface TextSectionProps {
  title?: string;
  paragraphs: string[];
  ctaButton?: {
    href: string;
    text: string;
    external?: boolean;
  };
}

export function TextSection({
  title,
  paragraphs,
  ctaButton,
}: TextSectionProps) {
  return (
    <section className="flex-1 space-y-4 text-base md:space-y-6">
      {title && (
        <h2 className="text-foreground text-2xl tracking-tight md:text-3xl">
          {title}
        </h2>
      )}
      <div className="text-muted-foreground max-w-3xl space-y-6">
        {paragraphs.map((paragraph, index) => (
          <p key={index}>{paragraph}</p>
        ))}
      </div>
      {ctaButton &&
        (ctaButton.external ? (
          <div className="mt-8">
            <Button size="lg" asChild>
              <a
                href={ctaButton.href}
                target="_blank"
                rel="noopener noreferrer"
              >
                {ctaButton.text}
              </a>
            </Button>
          </div>
        ) : (
          <div className="mt-8">
            <Button size="lg" asChild>
              <Link href={ctaButton.href}>{ctaButton.text}</Link>
            </Button>
          </div>
        ))}
    </section>
  );
}
