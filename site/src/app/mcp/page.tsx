import Link from "next/link";

import {
  Camera,
  CheckCircle2,
  Cloud,
  Eraser,
  FileDown,
  GitCompareArrows,
  ListChecks,
  ListTree,
  Monitor,
  Server,
  Send,
  Wand2,
  MessageSquare,
  Clock,
} from "lucide-react";

import { Background } from "@/components/background";
import { ModesComparison } from "@/components/blocks/modes-comparison";
import { CopyPrompt } from "@/components/site/copy-prompt";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  DESIGN_MODE_SKILL,
  MCP_SETUP_PROMPT,
  mcpClients,
} from "@/lib/mcp-guide";

const title = "Connect your AI app with MCP | Design Mode";
const description =
  "Set up Design Mode with a compatible AI app. Connect through Cloud, Local or Self-hosted MCP, test safely and read your browser design changes.";
export const metadata = {
  title: { absolute: title },
  description,
  alternates: { canonical: "https://designmode.app/mcp" },
  openGraph: {
    type: "website",
    title,
    description,
    url: "https://designmode.app/mcp",
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
    title,
    description,
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

const tools = [
  {
    name: "get_session_summary",
    icon: GitCompareArrows,
    access: "Read",
    description:
      "Check the connection, active sessions, tracked-item counts and hand-off marker.",
  },
  {
    name: "get_changes",
    icon: ListTree,
    access: "Read",
    description:
      "Fetch recorded styles, text, DOM edits, design tokens and comments, including item IDs and status.",
  },
  {
    name: "get_screenshot",
    icon: Camera,
    access: "Read",
    description:
      "Capture the visible page, or crop to one element or comment. Screenshots can include private page content.",
  },
  {
    name: "export_changes",
    icon: FileDown,
    access: "Read",
    description:
      "Read style changes as CSS, Tailwind, SCSS or JSX. This is separate from the extension's JSON export and Copy as Prompt.",
  },
  {
    name: "apply_changes",
    icon: Wand2,
    access: "Write",
    description:
      "Preview CSS changes in the browser. This does not edit your repository.",
  },
  {
    name: "set_change_status",
    icon: ListChecks,
    access: "Write",
    description:
      "Set specific tracked items to todo, in_progress or resolved using their IDs.",
  },
  {
    name: "mark_comment_resolved",
    icon: CheckCircle2,
    access: "Write",
    description: "Resolve or reopen one pinned comment using its commentId.",
  },
  {
    name: "clear_changes",
    icon: Eraser,
    access: "Destructive",
    description:
      "Clear the tracked session and revert its live-page edits. Ask before using it.",
  },
];

function Snippet({ children }: { children: string }) {
  return (
    <pre className="bg-ink text-ink-foreground mt-6 max-w-full overflow-x-auto rounded-xl p-4 font-mono text-base leading-relaxed">
      <code>{children}</code>
    </pre>
  );
}

export default function McpPage() {
  return (
    <>
      <Background>
        <section className="py-32" aria-labelledby="mcp-title">
          <div className="container max-w-5xl">
            <p className="text-muted-foreground text-base font-medium tracking-widest">
              DESIGN MODE + YOUR AI APP
            </p>
            <h1
              id="mcp-title"
              className="mt-6 text-4xl tracking-tight sm:text-5xl md:text-6xl"
            >
              Your browser changes.
              <br />
              In your agent’s hands.
            </h1>
            <p className="text-muted-foreground mt-6 max-w-2xl text-base leading-relaxed">
              Use Copy as prompt without a server, or connect a compatible
              client through Cloud, Local or Self-hosted MCP. The extension
              records browser changes; it does not grant repository access. This
              setup prompt can help your agent guide you through the connection.
            </p>
            <div className="bg-background/90 mt-10 rounded-xl border p-6 sm:p-8">
              <h2 className="text-xl font-semibold">Set up with your agent</h2>
              <CopyPrompt text={MCP_SETUP_PROMPT} />
            </div>
          </div>
        </section>
      </Background>

      <section className="py-32" aria-labelledby="what-mcp">
        <div className="container max-w-5xl">
          <h2 id="what-mcp" className="text-3xl tracking-tight md:text-4xl">
            What MCP makes possible
          </h2>
          <p className="text-muted-foreground mt-6 max-w-3xl text-base leading-relaxed">
            Model Context Protocol connects AI apps to external tools. Design
            Mode gives a connected agent tools to read your recorded browser
            edits, inspect screenshots and update their status. The agent still
            needs separate access to your repository to change source code.
            Review the{" "}
            <Link
              href="/docs/changes-tab"
              className="underline underline-offset-4"
            >
              recorded changes and copy formats
            </Link>
            , then use the{" "}
            <Link
              href="/blog/turn-visual-edits-into-precise-ai-prompts"
              className="underline underline-offset-4"
            >
              precise-prompts guide
            </Link>{" "}
            to add scope and acceptance checks.
          </p>
          <div className="mt-12 grid gap-8 rounded-xl border p-6 md:grid-cols-2 md:p-8">
            <div>
              <MessageSquare
                aria-hidden="true"
                className="text-primary size-8"
              />
              <h3 className="mt-6 text-2xl font-semibold">
                Ask your agent what’s left
              </h3>
              <p className="text-muted-foreground mt-4 leading-relaxed">
                When connected, your agent can fetch the latest changes when you
                ask. It reads updates when it makes a request or an explicitly
                enabled polling call—not through always-on background
                monitoring. Reading the list does not automatically edit your
                source.
              </p>
            </div>
            <div
              className="bg-muted/40 min-w-0 rounded-xl border"
              aria-label="Example conversation"
            >
              <p className="text-muted-foreground border-b px-6 py-4 text-base">
                Example conversation
              </p>
              <div className="space-y-6 p-6">
                <div>
                  <p className="text-base font-semibold">You</p>
                  <p className="mt-2">
                    What changes are left to complete in Design Mode?
                  </p>
                </div>
                <div>
                  <p className="text-base font-semibold">
                    Agent · read-only tool call
                  </p>
                  <code className="mt-2 block text-base">
                    get_changes({"{}"})
                  </code>
                </div>
                <div>
                  <p className="text-base font-semibold">Agent</p>
                  <p className="text-muted-foreground mt-2">
                    Two items are still pending in this example:
                  </p>
                  <ul className="mt-2 list-disc space-y-2 pl-5">
                    <li>Increase the hero heading’s line height.</li>
                    <li>
                      Address the comment on the primary button’s contrast.
                    </li>
                  </ul>
                  <p className="text-muted-foreground mt-3">
                    Would you like me to find these in your source?
                  </p>
                </div>
              </div>
            </div>
          </div>
          <p className="text-muted-foreground mt-8">
            No MCP support? Use{" "}
            <strong className="text-foreground">Copy as Prompt</strong> in the
            Changes tab and paste the Markdown into your app. It is a snapshot,
            not a live connection.
          </p>
        </div>
      </section>

      <section id="connect" className="py-32" aria-labelledby="connect-title">
        <div className="container max-w-5xl">
          <h2
            id="connect-title"
            className="text-3xl tracking-tight md:text-4xl"
          >
            Choose how to connect
          </h2>
          <div className="mt-10 grid gap-6 md:grid-cols-3">
            <Card>
              <CardContent className="space-y-4 p-6">
                <Cloud aria-hidden="true" className="size-6" />
                <h3 className="text-2xl font-semibold">
                  Cloud{" "}
                  <span className="text-muted-foreground text-base font-normal">
                    Default
                  </span>
                </h3>
                <p className="text-muted-foreground leading-relaxed">
                  No companion process to run. Open the extension’s MCP page
                  from its header chip, keep Cloud selected and create an
                  anonymous credential.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  Pair the agent using the same bearer token. No Design Mode
                  account or subscription verification; this is not an OAuth
                  sign-in flow.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="space-y-4 p-6">
                <Monitor aria-hidden="true" className="size-6" />
                <h3 className="text-2xl font-semibold">Local</h3>
                <p className="text-muted-foreground leading-relaxed">
                  Your MCP client starts the companion over stdio. It talks to
                  the extension on localhost, with no Design Mode relay traffic.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  Requires a local repository checkout and Node.js. Select Local
                  in the extension; the default bridge port is 9960.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="space-y-4 p-6">
                <Server aria-hidden="true" className="size-6" />
                <h3 className="text-2xl font-semibold">Self-hosted</h3>
                <p className="text-muted-foreground leading-relaxed">
                  Run the Node.js + Redis relay on infrastructure you operate.
                  Configure its base URL in the extension and use that
                  deployment’s /mcp endpoint in your agent.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  Use a credential registered with that relay—not a hosted Cloud
                  token.
                </p>
              </CardContent>
            </Card>
          </div>
          <p className="mt-8 font-medium">Cloud endpoint · Streamable HTTP</p>
          <Snippet>{"https://mcp.designmode.app/mcp"}</Snippet>
          <p className="text-muted-foreground mt-4">
            Use the apex host and the /mcp path. The extension’s event-stream
            URL is not the agent endpoint. Keep bearer tokens out of chat,
            screenshots, shell history and committed configuration.
          </p>
          <Accordion type="single" collapsible className="mt-8">
            <AccordionItem value="local">
              <AccordionTrigger className="text-base">
                Local setup from source
              </AccordionTrigger>
              <AccordionContent className="text-base leading-relaxed">
                <p>
                  Review and clone the{" "}
                  <a
                    href="https://github.com/SandeepBaskaran/design-mode"
                    className="underline underline-offset-4"
                  >
                    source repository
                  </a>
                  , then run npm ci at its root to install locked dependencies.
                  Configure your client to launch the following command, using
                  your absolute checkout path:
                </p>
                <Snippet>
                  {
                    "npm start --prefix /absolute/path/to/design-mode/packages/mcp-local"
                  }
                </Snippet>
                <p className="mt-4">
                  For a stdio JSON entry, use command <code>npm</code> and args{" "}
                  <code>
                    [&quot;start&quot;, &quot;--prefix&quot;,
                    &quot;/absolute/path/to/design-mode/packages/mcp-local&quot;]
                  </code>
                  . The client owns this process. The published agent CLI is{" "}
                  <code>@designmode-app/cli</code> (<code>designmode-app</code>
                  ). It does not start this bridge and it does not send
                  analytics. If another app owns port 9960, set DM_PORT in the
                  companion’s environment and match the extension’s Local port;
                  never kill a foreign process.
                </p>
              </AccordionContent>
            </AccordionItem>
            <AccordionItem value="self">
              <AccordionTrigger className="text-base">
                Self-hosted setup
              </AccordionTrigger>
              <AccordionContent className="text-base leading-relaxed">
                <p>
                  Deploy packages/mcp-cloud with Redis using its{" "}
                  <a
                    href="https://github.com/SandeepBaskaran/design-mode/tree/main/packages/mcp-cloud"
                    className="underline underline-offset-4"
                  >
                    deployment guide
                  </a>
                  . Keep TLS and bearer authentication enabled. Set the
                  extension’s relay base URL, register an anonymous credential
                  there, then replace only the URL in your client example with{" "}
                  <code>https://your-relay.example/mcp</code> and provide that
                  relay’s token locally.
                </p>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        </div>
      </section>

      <section className="py-32" aria-labelledby="apps-title">
        <div className="container max-w-5xl">
          <h2 className="text-2xl tracking-tight md:text-3xl">
            Mode comparison
          </h2>
          <p className="text-muted-foreground mt-2 max-w-2xl">
            Setup, MCP transport, client requirements and cost across the three
            modes. Local keeps this MCP transport on localhost; it is not a
            no-egress guarantee for the coding client or other enabled features.
            See the{" "}
            <Link href="/privacy" className="underline underline-offset-4">
              privacy disclosure
            </Link>{" "}
            for the separate data flows.
          </p>
        </div>
        <ModesComparison />
        <div className="container max-w-5xl">
          <h2 id="apps-title" className="text-3xl tracking-tight md:text-4xl">
            Configure your AI app
          </h2>
          <p className="text-muted-foreground mt-6 max-w-3xl text-base leading-relaxed">
            These Cloud examples use the client’s own configuration format.
            Merge the entry—do not replace other servers. Enter your token
            locally using a protected input or a securely supplied environment
            variable.
          </p>
          <Accordion
            type="single"
            collapsible
            defaultValue="Claude Code"
            className="mt-8"
          >
            {mcpClients.map((client) => (
              <AccordionItem key={client.name} value={client.name}>
                <AccordionTrigger className="text-base">
                  {client.name}
                </AccordionTrigger>
                <AccordionContent className="text-base leading-relaxed">
                  <p className="font-medium">{client.location}</p>
                  <p className="text-muted-foreground mt-4">
                    {client.description}
                  </p>
                  <Snippet>{client.code}</Snippet>
                  <a
                    href={client.source}
                    className="mt-6 inline-block underline underline-offset-4"
                  >
                    Official {client.name} MCP documentation →
                  </a>
                </AccordionContent>
              </AccordionItem>
            ))}
            <AccordionItem value="other">
              <AccordionTrigger className="text-base">
                Claude Desktop, Windsurf, Cline and other apps
              </AccordionTrigger>
              <AccordionContent className="text-base leading-relaxed">
                <p>
                  MCP support alone is not enough: Cloud and Self-hosted require
                  Streamable HTTP and a custom Authorization bearer header. A
                  connector that only supports OAuth or has no custom-header
                  input cannot connect directly with this token. Do not try an
                  OAuth login as a substitute. Check your installed client’s
                  current documentation; choose Local if it supports stdio, or
                  use Copy as Prompt.
                </p>
                <p className="mt-4">
                  For Claude Desktop, use its documented local-server
                  configuration for Local mode rather than assuming the remote
                  Connectors UI accepts bearer headers. Configuration files and
                  reload steps differ by app.
                </p>
                <p className="mt-4">
                  <Link
                    href="/docs/mcp-setup"
                    className="underline underline-offset-4"
                  >
                    Detailed setup guide →
                  </Link>
                </p>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
          <p className="text-muted-foreground mt-6 text-base">
            Examples checked against first-party documentation on 29 September
            2026. Client versions and organisation policies can change
            availability. Use the{" "}
            <Link
              href="/docs/mcp-setup"
              className="underline underline-offset-4"
            >
              client setup reference
            </Link>{" "}
            for configuration details and{" "}
            <Link
              href="/docs/troubleshooting"
              className="underline underline-offset-4"
            >
              troubleshooting
            </Link>{" "}
            when a connection check fails.
          </p>
        </div>
      </section>

      <section className="py-32" aria-labelledby="tools-title">
        <div className="container max-w-5xl">
          <h2 id="tools-title" className="text-3xl tracking-tight md:text-4xl">
            Tools your agent can use
          </h2>
          <p className="text-muted-foreground mt-6 max-w-3xl text-base">
            Cloud, Local and Self-hosted expose these eight session tools. Read
            the discovered schema before each new kind of call.
          </p>
          <div className="mt-10 grid gap-4 md:grid-cols-2">
            {tools.map((tool) => {
              const Icon = tool.icon;
              return (
                <Card key={tool.name}>
                  <CardContent className="p-6">
                    <div className="flex items-center gap-3">
                      <Icon aria-hidden="true" className="size-5 shrink-0" />
                      <h3 className="min-w-0 font-mono text-base font-semibold break-words">
                        {tool.name}
                      </h3>
                    </div>
                    <p className="text-muted-foreground mt-3 text-base">
                      {tool.access} · All modes
                    </p>
                    <p className="text-muted-foreground mt-3 leading-relaxed">
                      {tool.description}
                    </p>
                  </CardContent>
                </Card>
              );
            })}
          </div>
          <Card className="mt-4">
            <CardContent className="p-6">
              <div className="flex items-center gap-3">
                <Clock aria-hidden="true" className="size-5" />
                <h3 className="font-mono text-base font-semibold">
                  wait_for_handoff
                </h3>
              </div>
              <p className="text-muted-foreground mt-3 text-base">
                Opt-in session control · Local only
              </p>
              <p className="text-muted-foreground mt-3 leading-relaxed">
                Wait up to 20 seconds for an explicit Send to Agent. Each
                feedback response contains an immutable snapshot. Only repeat
                calls after the user chooses live rounds; stop on stopped, busy
                or an error. Cloud and Self-hosted use one-shot reads.
              </p>
            </CardContent>
          </Card>
        </div>
      </section>

      <section className="py-32" aria-labelledby="reads-title">
        <div className="container max-w-5xl">
          <h2 id="reads-title" className="text-3xl tracking-tight md:text-4xl">
            Read progressively, act on a bounded scope
          </h2>
          <ol className="text-muted-foreground mt-8 max-w-3xl list-decimal space-y-6 pl-5 text-base leading-relaxed">
            <li>
              <strong className="text-foreground">Start small.</strong> Get the
              session summary before fetching a full change report.
            </li>
            <li>
              <strong className="text-foreground">
                Fetch once, select locally.
              </strong>{" "}
              Call get_changes with no arguments and work from its unresolved
              items. It has no server-side pagination, status filter or
              max_chars parameter—do not invent them.
            </li>
            <li>
              <strong className="text-foreground">
                Zoom in only when needed.
              </strong>{" "}
              Request a screenshot with one returned selector or elementId, or a
              commentId for a pinned region. Export one supported format if
              useful.
            </li>
            <li>
              <strong className="text-foreground">Keep writes explicit.</strong>{" "}
              Update only selected item IDs after implementing and verifying
              source changes. Omitting ids from set_change_status targets
              everything.
            </li>
          </ol>
          <Snippet>
            {
              'get_session_summary({})\nget_changes({})\nget_screenshot({ "commentId": "<id from get_changes>" })\nexport_changes({ "format": "css" })'
            }
          </Snippet>
          <p className="text-muted-foreground mt-6">
            For opt-in Local live rounds, start wait_for_handoff with the exact
            pageUrl and timeoutMs ≤ 20000. Continue only with the returned
            sessionId and after cursor. Never fabricate a session ID or treat
            later edits as part of an earlier snapshot.
          </p>
        </div>
      </section>

      <Background variant="bottom">
        <section className="py-32" aria-labelledby="skill-title">
          <div className="container max-w-5xl">
            <Send aria-hidden="true" className="size-8" />
            <h2
              id="skill-title"
              className="mt-6 text-3xl tracking-tight md:text-4xl"
            >
              Give your agent the workflow
            </h2>
            <p className="text-muted-foreground mt-6 max-w-3xl text-base leading-relaxed">
              The Design Mode agent skill covers setup, safe connection checks
              and a scoped design-to-code workflow. Download the Markdown and
              follow your app’s skill-install instructions, or ask a capable
              agent to read it. Reading a skill does not install an MCP
              connection or grant tool permissions.
            </p>
            <div className="mt-8 flex flex-wrap gap-4">
              <Button asChild>
                <a href={DESIGN_MODE_SKILL} download="SKILL.md">
                  <FileDown aria-hidden="true" className="size-4" />
                  Download agent skill
                </a>
              </Button>
              <Button asChild variant="outline">
                <a href={DESIGN_MODE_SKILL}>Read SKILL.md</a>
              </Button>
            </div>
            <p className="text-muted-foreground mt-6">
              Discoverable through the{" "}
              <a
                href="/.well-known/agent-skills/index.json"
                className="underline underline-offset-4"
              >
                skill index
              </a>{" "}
              and{" "}
              <a href="/llms.txt" className="underline underline-offset-4">
                llms.txt
              </a>
              . The existing{" "}
              <a
                href="/design-mode.md"
                className="underline underline-offset-4"
              >
                command prompt
              </a>{" "}
              remains available separately.
            </p>
            <p className="text-muted-foreground mt-8 text-base">
              Local keeps Design Mode MCP traffic on your machine. Cloud and
              Self-hosted relay page data; queue expiry is best-effort.{" "}
              <Link href="/privacy" className="underline underline-offset-4">
                Read the privacy disclosure →
              </Link>
            </p>
          </div>
        </section>
      </Background>
    </>
  );
}
