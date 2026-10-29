import Image from "next/image";
import Link from "next/link";

import Marquee from "react-fast-marquee";

import { withNavRef } from "@/lib/nav-ref";
import { cn } from "@/lib/utils";

type Company = {
  name: string;
  logo: string;
  width: number;
  height: number;
  href: string;
};

export const Logos = () => {
  const topRowCompanies: Company[] = [
    {
      name: "OpenAI",
      logo: "/logos/openai.svg",
      width: 100,
      height: 26,
      href: "https://openai.com",
    },
    {
      name: "Claude",
      logo: "/logos/claude.svg",
      width: 120,
      height: 30,
      href: "https://www.anthropic.com/claude",
    },
    {
      name: "Gemini",
      logo: "/logos/gemini.svg",
      width: 118,
      height: 28,
      href: "https://gemini.google.com",
    },
    {
      name: "Perplexity",
      logo: "/logos/perplexity.svg",
      width: 141,
      height: 32,
      href: "https://perplexity.ai",
    },
  ];

  const bottomRowCompanies: Company[] = [
    {
      name: "Cursor",
      logo: "/logos/cursor.svg",
      width: 108,
      height: 28,
      href: "https://cursor.com",
    },
    {
      name: "Windsurf",
      logo: "/logos/windsurf.svg",
      width: 124,
      height: 28,
      href: "https://windsurf.com",
    },
    {
      name: "Antigravity",
      logo: "/logos/antigravity.svg",
      width: 140,
      height: 28,
      href: "https://antigravity.google",
    },
  ];

  return (
    <section className="overflow-hidden pb-28 lg:pb-32">
      <div className="container space-y-10 lg:space-y-16">
        <div className="text-center">
          <h2 className="mb-4 text-2xl text-balance md:text-3xl lg:text-4xl">
            Talks to every AI tool that speaks MCP.
            <br className="max-md:hidden" />
            <span className="text-muted-foreground">
              Drop your edits straight into Claude, Cursor, or any agent that
              supports the protocol.
            </span>
          </h2>
        </div>

        <div className="flex w-full flex-col items-center gap-8">
          <LogoRow companies={topRowCompanies} gridClassName="grid-cols-4" />
          <LogoRow
            companies={bottomRowCompanies}
            gridClassName="grid-cols-3"
            direction="right"
          />
        </div>
      </div>
    </section>
  );
};

type LogoRowProps = {
  companies: Company[];
  gridClassName: string;
  direction?: "left" | "right";
};

const LogoRow = ({ companies, gridClassName, direction }: LogoRowProps) => {
  return (
    <>
      {/* Desktop static version */}
      <div className="hidden md:block">
        <div
          className={cn(
            "grid items-center justify-items-center gap-x-20 lg:gap-x-28",
            gridClassName,
          )}
        >
          {companies.map((company, index) => (
            <Link href={withNavRef(company.href)} target="_blank" key={index}>
              <Image
                src={company.logo}
                alt={`${company.name} logo`}
                width={company.width}
                height={company.height}
                className="h-8 w-auto object-contain opacity-50 brightness-0 transition hover:opacity-100"
              />
            </Link>
          ))}
        </div>
      </div>

      {/* Mobile marquee version */}
      <div className="md:hidden">
        <Marquee direction={direction} pauseOnHover>
          {companies.map((company, index) => (
            <Link
              href={withNavRef(company.href)}
              target="_blank"
              key={index}
              className="mx-8 inline-block"
            >
              <Image
                src={company.logo}
                alt={`${company.name} logo`}
                width={company.width}
                height={company.height}
                className="h-8 w-auto object-contain opacity-50 brightness-0 transition hover:opacity-100"
              />
            </Link>
          ))}
        </Marquee>
      </div>
    </>
  );
};
