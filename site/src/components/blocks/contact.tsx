import React from "react";

import Link from "next/link";

import { Mail, MessageCircle } from "lucide-react";

import { withNavRef } from "@/lib/nav-ref";

const REPO_URL = "https://github.com/SandeepBaskaran/design-mode";
const EMAIL = "hello@sandeepbaskaran.com";

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
      className={className}
    >
      <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}

export function ContactHero() {
  return (
    <div className="container max-w-5xl">
      <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
        Get in touch
      </h1>
      <p className="text-muted-foreground mt-4 max-w-3xl text-base">
        Bug? Feature idea? Sponsorship question? Pick the channel that fits —
        email for off-the-record, GitHub for everything else.
      </p>
    </div>
  );
}

export function ContactChannels() {
  return (
    <div className="container max-w-5xl">
      <div className="grid gap-6 sm:grid-cols-2">
        <div className="bg-card border-border rounded-2xl border p-6 shadow-sm">
          <h2 className="text-foreground flex items-center gap-2 text-base font-semibold">
            <Mail className="size-6" /> Email
          </h2>
          <div className="mt-4">
            <Link
              href={`mailto:${EMAIL}`}
              className="text-muted-foreground hover:text-foreground"
            >
              {EMAIL}
            </Link>
            <p className="text-muted-foreground mt-2 text-base leading-relaxed">
              Best for security disclosures, partnerships, and anything off the
              record.
            </p>
          </div>
        </div>

        <div className="bg-card border-border rounded-2xl border p-6 shadow-sm">
          <h2 className="text-foreground flex items-center gap-2 text-base font-semibold">
            <GithubIcon className="size-6" /> GitHub
          </h2>
          <div className="mt-4 space-y-2">
            <Link
              href={withNavRef(`${REPO_URL}/issues/new/choose`)}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground block"
            >
              File an issue ↗
            </Link>
            <Link
              href={withNavRef(`${REPO_URL}/discussions`)}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground block"
            >
              Start a discussion ↗
            </Link>
            <Link
              href={withNavRef(`${REPO_URL}/compare`)}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground block"
            >
              Open a pull request ↗
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}

export function ContactReports() {
  return (
    <div className="container max-w-3xl">
      <div className="grid gap-6 sm:grid-cols-2">
        <div className="bg-card border-border rounded-2xl border p-6 shadow-sm">
          <h2 className="flex items-center gap-2 text-2xl font-semibold">
            <MessageCircle className="size-6" /> Where bug reports go
          </h2>
          <p className="text-muted-foreground mt-4 text-base leading-relaxed">
            For reproducible bugs, the issue template is the right place. Open
            the Help panel inside the side panel (the{" "}
            <span className="font-mono text-base">?</span> icon next to the gear),
            click <span className="font-mono text-base">Copy diagnostics</span>,
            paste that into the issue, and you've handed us 80% of what we need
            to investigate.
          </p>
        </div>
        <div className="bg-card border-border rounded-2xl border p-6 shadow-sm">
          <h2 className="flex items-center gap-2 text-2xl font-semibold">
            <Mail className="size-6" /> Security
          </h2>
          <p className="text-muted-foreground mt-4 text-base leading-relaxed">
            Please don't file security reports as public issues. Email{" "}
            <Link
              href={`mailto:${EMAIL}`}
              className="underline underline-offset-8"
            >
              {EMAIL}
            </Link>{" "}
            with the details — full disclosure policy lives in{" "}
            <Link
              href={withNavRef(`${REPO_URL}/blob/main/SECURITY.md`)}
              target="_blank"
              rel="noopener noreferrer"
              className="underline underline-offset-8"
            >
              SECURITY.md
            </Link>
            .
          </p>
        </div>
      </div>
    </div>
  );
}
