import Image from "next/image";
import Link from "next/link";

import { ArrowUpRight } from "lucide-react";

import { withNavRef } from "@/lib/nav-ref";

const REPO_URL = withNavRef("https://github.com/SandeepBaskaran/design-mode");

const SPONSORS_URL = withNavRef("https://github.com/sponsors/SandeepBaskaran");
const PRODUCT_HUNT_URL = withNavRef(
  "https://www.producthunt.com/products/design-mode",
);
const AUTHOR_URL = withNavRef("https://sandeepbaskaran.com");

export function Footer() {
  const groups = [
    {
      title: "Product",
      links: [
        { name: "Features", href: "/features" },
        { name: "Try the demo", href: "/demo" },
        { name: "Connect your agent", href: "/mcp" },
        { name: "Use cases", href: "/use-cases" },

      ],
    },
    {
      title: "Resources",
      links: [
        { name: "Documentation", href: "/docs" },
        { name: "Changelog", href: "/changelog" },
        { name: "Compare", href: "/compare" },
        { name: "Blog", href: "/blog" },

      ],
    },
    {
      title: "Project",
      links: [
        { name: "About", href: "/about" },
        { name: "Contact", href: "/contact" },
        { name: "Privacy", href: "/privacy" },
        { name: "FAQ", href: "/faq" },

      ],
    },
    {
      title: "Community",
      links: [
        { name: "GitHub", href: REPO_URL },

        { name: "Product Hunt", href: PRODUCT_HUNT_URL },
        { name: "Sponsor", href: SPONSORS_URL },
      ],
    },
  ];

  return (
    <footer className="border-t py-16 md:pt-24 md:pb-8">
      <div className="container grid max-w-[1200px] gap-16 lg:grid-cols-[1.5fr_3fr] lg:gap-x-16 lg:gap-y-16">
        <div className="flex flex-col items-center text-center md:items-start md:text-left">
          <Link
            href="/"
            className="focus-visible:outline-ring inline-flex items-center gap-2 rounded-lg focus-visible:outline-2 focus-visible:outline-offset-4"
          >
            <Image
              src="/brand-icon.png"
              width={40}
              height={40}
              alt=""
              unoptimized
            />
            <span className="text-2xl font-semibold tracking-tight">
              Design Mode
            </span>
          </Link>
          <p className="mt-4 max-w-xs text-base leading-relaxed text-[#444444]">
            Make the change you want to see.
            <br />
            Give your agent the details.
          </p>
          <p className="text-muted-foreground mt-8 text-base">
            Free to install. Open source under MIT.
          </p>
        </div>
        <nav
          aria-label="Footer"
          className="grid grid-cols-2 gap-x-4 gap-y-8 md:grid-cols-4 md:gap-x-8"
        >
          {groups.map((group) => (
            <div key={group.title}>
              <h2 className="text-muted-foreground mb-4 text-base font-medium">
                {group.title}
              </h2>
              <ul className="space-y-0">
                {group.links.map((item) => (
                  <li key={item.name}>
                    <Link
                      href={item.href}
                      {...(item.href.startsWith("https://")
                        ? { target: "_blank", rel: "noopener noreferrer" }
                        : {})}
                      className="hover:text-primary focus-visible:outline-ring inline-flex items-center gap-2 rounded-lg py-1 text-base leading-snug transition-colors focus-visible:outline-2 focus-visible:outline-offset-4"
                    >
                      {item.name}
                      {item.href.startsWith("https://") && (
                        <ArrowUpRight
                          className="size-4 shrink-0"
                          aria-hidden="true"
                        />
                      )}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
        <div className="text-muted-foreground col-span-full flex w-full flex-col items-center gap-2 text-center text-base md:flex-row md:justify-between md:text-left">
          <p>© {new Date().getFullYear()} Design Mode</p>
          <p>
            Made by{" "}
            <a
              href={AUTHOR_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-primary focus-visible:outline-ring rounded-lg underline-offset-4 hover:underline focus-visible:outline-2 focus-visible:outline-offset-4"
            >
              Sandeep Baskaran
            </a>
          </p>
        </div>
      </div>
    </footer>
  );
}
