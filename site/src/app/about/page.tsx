import { Background } from "@/components/background";
import { WhyThisExists } from "@/components/blocks/about";
import { AboutHero } from "@/components/blocks/about-hero";
import { DashedLine } from "@/components/dashed-line";
import { JsonLd, personSchema } from "@/components/site/json-ld";

export const metadata = {
  title: { absolute: "Why Design Mode exists | Design Mode" },
  description:
    "Why Sandeep Baskaran built an open-source browser visual editor that turns rendered-page changes into precise coding-agent instructions.",
  keywords: [
    "Sandeep Baskaran",
    "design engineer",
    "Design Mode creator",
    "open source design tool",
    "why Design Mode",
  ],
  alternates: { canonical: "https://designmode.app/about" },
  openGraph: {
    type: "website",
    title: "Why Design Mode exists | Design Mode",
    description:
      "Why Sandeep Baskaran built an open-source browser visual editor that turns rendered-page changes into precise coding-agent instructions.",
    url: "https://designmode.app/about",
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "Design Mode — The visual editor for all your agent’s work",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Why Design Mode exists | Design Mode",
    description:
      "Why Sandeep Baskaran built an open-source browser visual editor that turns rendered-page changes into precise coding-agent instructions.",
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "Design Mode — The visual editor for all your agent’s work",
      },
    ],
  },
};

export default function AboutPage() {
  return (
    <>
      <JsonLd data={personSchema} />
      {/* Hero — yellow background slab */}
      <Background>
        <section className="py-12 lg:py-16">
          <AboutHero />
        </section>
      </Background>

      {/* Why this exists — yellow background slab */}
      <Background variant="bottom">
        <section className="py-20 lg:py-28">
          <DashedLine className="container max-w-5xl" />
          <div className="mt-16">
            <WhyThisExists />
          </div>
        </section>
      </Background>
    </>
  );
}
