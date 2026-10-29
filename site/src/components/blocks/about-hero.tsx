import Link from "next/link";

export function AboutHero() {
  return (
    <section>
      <div className="container max-w-5xl">
        <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
          About Sandeep
        </h1>
        <p className="text-muted-foreground mt-4 max-w-5xl text-base">
          Sandeep Baskaran is a Senior UX Designer based in Bengaluru, currently
          working in IBM&apos;s Z Infrastructure organisation where he helps
          enterprises modernize legacy systems. He builds in the open —
          automating workflows, writing about design, AI, and productivity,
          mentoring aspiring designers, and leading design communities in
          Chennai and Bengaluru. He also builds TypeScript and React tools for
          agent-assisted product work.{" "}
          <Link href="/" className="underline underline-offset-4">
            Design Mode
          </Link>{" "}
          is his independent, MIT-licensed browser visual editor: it previews
          changes on rendered pages and records a specification, while an
          authorised developer or coding agent implements the source. Read the{" "}
          <Link href="/changelog" className="underline underline-offset-4">
            release history
          </Link>{" "}
          and the{" "}
          <Link
            href="/blog/why-we-built-an-mcp-server-for-design-edits"
            className="underline underline-offset-4"
          >
            MCP design rationale
          </Link>
          .
        </p>
      </div>
    </section>
  );
}
