import React from "react";

import { Background } from "@/components/background";
import {
  ContactChannels,
  ContactHero,
  ContactReports,
} from "@/components/blocks/contact";
import { DashedLine } from "@/components/dashed-line";

export const metadata = {
  title: { absolute: "Contact and support | Design Mode" },
  description:
    "Get in touch about Design Mode — email, GitHub issues, Discussions, and security disclosure. Bug? Feature idea? Sponsorship question?",
  keywords: [
    "Design Mode contact",
    "bug report Design Mode",
    "Design Mode support",
    "Design Mode security disclosure",
  ],
  alternates: { canonical: "https://designmode.app/contact" },
  openGraph: {
    type: "website",
    title: "Contact and support | Design Mode",
    description:
      "Get in touch about Design Mode — email, GitHub issues, Discussions, and security disclosure. Bug? Feature idea? Sponsorship question?",
    url: "https://designmode.app/contact",
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
    title: "Contact and support | Design Mode",
    description:
      "Get in touch about Design Mode — email, GitHub issues, Discussions, and security disclosure. Bug? Feature idea? Sponsorship question?",
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

const Page = () => {
  return (
    <>
      {/* Hero — yellow background slab */}
      <Background>
        <section className="py-12 lg:py-16">
          <ContactHero />
        </section>
      </Background>

      {/* Middle — channels */}
      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-5xl" />
        <div className="mt-12">
          <ContactChannels />
        </div>
      </section>

      {/* Bottom — reports + security in yellow slab */}
      <Background variant="bottom">
        <section className="py-20 lg:py-28">
          <DashedLine className="container max-w-5xl" />
          <div className="mt-16">
            <ContactReports />
          </div>
        </section>
      </Background>
    </>
  );
};

export default Page;
