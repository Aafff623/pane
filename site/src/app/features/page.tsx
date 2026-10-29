import Link from "next/link";

import {
  AlignLeft,
  ArrowDown,
  ArrowUp,
  Bookmark,
  Camera,
  Copy,
  CornerUpLeft,
  Eye,
  HeartHandshake,
  HelpCircle,
  Layers,
  Layers3,
  MessageCircle,
  MoveHorizontal,
  Pause,
  Plug,
  Send,
  Settings as SettingsIcon,
  SunMoon,
  Sparkles,
  Square,
  Type,
} from "lucide-react";

import { Background } from "@/components/background";
import { DashedLine } from "@/components/dashed-line";
import { Card, CardContent } from "@/components/ui/card";

export const metadata = {
  title: { absolute: "Browser visual editor features | Design Mode" },
  description:
    "Inspect and edit a rendered webpage with Layers, visual controls, structured change history, Copy as Prompt and MCP agent hand-off.",
  keywords: [
    "Design Mode features",
    "side panel design editor",
    "Layers tab",
    "Changes tab",
    "visual CSS editor",
    "DOM tree editor",
    "in-browser design tool",
    "browser extension for designers",
    "Copy Prompt",
    "Send to Agent",
  ],
  alternates: { canonical: "https://designmode.app/features" },
  openGraph: {
    type: "website",
    title: "Browser visual editor features | Design Mode",
    description:
      "Inspect and edit a rendered webpage with Layers, visual controls, structured change history, Copy as Prompt and MCP agent hand-off.",
    url: "https://designmode.app/features",
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
    title: "Browser visual editor features | Design Mode",
    description:
      "Inspect and edit a rendered webpage with Layers, visual controls, structured change history, Copy as Prompt and MCP agent hand-off.",
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

const headerIcons = [
  {
    icon: Plug,
    title: "MCP status",
    description:
      "Chip showing whether the MCP server is offline, running, or has an agent attached. Click it to open the dedicated MCP page.",
  },
  {
    icon: SunMoon,
    title: "Theme",
    description: "Toggle the side panel between system / light / dark.",
  },
  {
    icon: HeartHandshake,
    title: "Contribute",
    description:
      "Star the repo, share, sponsor — full overlay with low-friction ways to support the project.",
  },
  {
    icon: HelpCircle,
    title: "Help",
    description:
      "Quick links to docs, privacy, and a one-click “Copy diagnostics” for bug reports.",
  },
  {
    icon: SettingsIcon,
    title: "Settings",
    description:
      "Theme, colour format, capture mode, inspector colours, and Chrome launch surface. MCP configuration lives on its own page, opened from the MCP chip.",
  },
];

const actionItems = [
  {
    icon: ArrowUp,
    title: "Parent / Child",
    description: "Walk selection up and down the DOM tree.",
  },
  {
    icon: ArrowDown,
    title: "Duplicate / Delete",
    description:
      "DOM mutations recorded as undoable changes. Deleting a multi-selection records and restores every layer in structural order.",
  },
  {
    icon: MessageCircle,
    title: "Comment",
    description: "Drop a numbered sticky-pin on the selected element.",
  },
  {
    icon: Pause,
    title: "Freeze animations",
    description:
      "Pause CSS and Web Animations API animations and playing videos; suppress transitions. Custom JavaScript animation loops may continue.",
  },
  {
    icon: Camera,
    title: "Screenshot",
    description: "Capture the visible tab to clipboard, download, or both.",
  },
  {
    icon: Bookmark,
    title: "Presets",
    description:
      "Save supported style properties as presets and reapply them. Presets do not capture every editor action or page state.",
  },
  {
    icon: CornerUpLeft,
    title: "Undo / Redo",
    description:
      "Step backward and forward through every style, text, and DOM change.",
  },
];

const designSections = [
  { icon: AlignLeft, label: "Position" },
  { icon: Layers, label: "Layout" },
  { icon: Type, label: "Typography" },
  { icon: Eye, label: "Fill" },
  { icon: Square, label: "Stroke" },
  { icon: Sparkles, label: "Effects" },
  { icon: MoveHorizontal, label: "Motion" },
  { icon: Layers3, label: "Layout Guides" },
];

export default function FeaturesPage() {
  return (
    <>
      {/* Hero — yellow background confined to this slab */}
      <Background>
        <section className="py-12 lg:py-16">
          <div className="container max-w-5xl">
            <h1 className="text-3xl tracking-tight sm:text-4xl md:text-5xl">
              Every control, where you need it
            </h1>
            <p className="text-muted-foreground mt-4 max-w-3xl text-base">
              The Design Mode side panel is split into three rows. A header row
              to pick what you're working on, three middle panels for the actual
              editing, and a bottom row for handing changes to your AI coding
              agent. These are browser previews, not source edits or published
              changes. Try the{" "}
              <Link href="/demo" className="underline underline-offset-4">
                guided demo
              </Link>{" "}
              on a supported page.
            </p>
          </div>
        </section>
      </Background>

      {/* Middle — plain background */}
      <section className="py-16 lg:py-20">
        <DashedLine className="container max-w-5xl" />

        {/* Section 1: Header row + Action toolbar (one merged card) */}
        <div className="container mt-16 max-w-5xl">
          <div className="mb-8 flex items-baseline gap-4">
            <span className="text-muted-foreground font-mono text-base tracking-wide uppercase">
              Row 1
            </span>
            <h2 className="text-2xl tracking-tight md:text-3xl">
              Top of the panel
            </h2>
          </div>
          <p className="text-muted-foreground mb-10 max-w-2xl">
            Five icons set up your session and surface the overlays (Contribute,
            Help, Settings). Just below them, an action toolbar puts the
            most-used DOM and session mutations one click away.
          </p>

          <Card>
            <CardContent className="grid gap-8 p-6 md:grid-cols-2 md:p-8">
              <div>
                <h3 className="text-foreground mb-4 text-base font-semibold">
                  Header icons
                </h3>
                <ul className="space-y-4">
                  {headerIcons.map((item) => {
                    const Icon = item.icon;
                    return (
                      <li key={item.title} className="flex items-start gap-4">
                        <Icon className="text-foreground mt-0 size-4 shrink-0" />
                        <div>
                          <div className="text-foreground text-base font-semibold">
                            {item.title}
                          </div>
                          <div className="text-muted-foreground text-base leading-relaxed">
                            {item.description}
                          </div>
                        </div>
                      </li>
                    );
                  })}
                </ul>
              </div>

              <div>
                <h3 className="text-foreground mb-4 text-base font-semibold">
                  Action toolbar
                </h3>
                <ul className="space-y-4">
                  {actionItems.map((item) => {
                    const Icon = item.icon;
                    return (
                      <li key={item.title} className="flex items-start gap-4">
                        <Icon className="text-foreground mt-0 size-4 shrink-0" />
                        <div>
                          <div className="text-foreground text-base font-semibold">
                            {item.title}
                          </div>
                          <div className="text-muted-foreground text-base leading-relaxed">
                            {item.description}
                          </div>
                        </div>
                      </li>
                    );
                  })}
                </ul>
              </div>
            </CardContent>
          </Card>
        </div>

        <DashedLine className="container mt-20 max-w-5xl" />

        {/* Section 2: Three middle panels */}
        <div className="container mt-16 max-w-5xl">
          <div className="mb-8 flex items-baseline gap-4">
            <span className="text-muted-foreground font-mono text-base tracking-wide uppercase">
              Row 2
            </span>
            <h2 className="text-2xl tracking-tight md:text-3xl">
              Three panels — the editing surface
            </h2>
          </div>
          <p className="text-muted-foreground mb-10 max-w-2xl">
            Layers, Design, and Changes. Each tab is its own scrollable surface;
            switching keeps the selection alive. On the canvas itself, hovering
            measures spacing between elements and the selection gets eight
            drag-to-resize handles — every resize ships as a change like any
            other edit.
          </p>

          <div className="grid gap-6 md:grid-cols-3">
            <Card>
              <CardContent className="flex h-full flex-col gap-4 p-6">
                <Layers className="text-foreground size-6" />
                <h3 className="text-2xl font-semibold">Layers</h3>
                <p className="text-muted-foreground text-base leading-relaxed">
                  The accessible DOM tree of the page you're editing. Search,
                  expand/collapse, multi-select, drag rows to rearrange the
                  actual document, toggle visibility with an eye icon per row.
                </p>
                <ul className="text-muted-foreground mt-2 space-y-2 text-base">
                  <li>• Search by tag, class, id, or text</li>
                  <li>• Drag to reorder · drop on a row to nest</li>
                  <li>• Multi-select for batch edits</li>
                  <li>• Indentation guides + collapse</li>
                </ul>
              </CardContent>
            </Card>

            <Card className="outline-primary outline-4">
              <CardContent className="flex h-full flex-col gap-4 p-6">
                <Sparkles className="text-foreground size-6" />
                <h3 className="text-2xl font-semibold">Design</h3>
                <p className="text-muted-foreground text-base leading-relaxed">
                  Controls grouped by editing task — sliders, colour pickers,
                  segmented buttons — not a textarea of CSS. Motion leads with
                  trigger-first interaction cards (Hover / Press / Focus /
                  Appear / Loop / Scroll). Inspect authored hover, focus, and
                  active states; force-preview them, or overlay computed
                  flex/grid tracks. Any field bound to a design-system token
                  shows a ◆ badge for swapping or editing it when the token can
                  be detected. Width and height show authored CSS separately
                  from the computed measurement. Rich text preserves attributed
                  spans, icons and media while exposing safe link destinations.
                </p>
                <ul className="mt-2 grid grid-cols-1 gap-2 text-base">
                  {designSections.map((s) => {
                    const Icon = s.icon;
                    return (
                      <li
                        key={s.label}
                        className="text-foreground/90 flex items-center gap-2"
                      >
                        <Icon className="text-foreground/70 size-4" />
                        {s.label}
                      </li>
                    );
                  })}
                </ul>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="flex h-full flex-col gap-4 p-6">
                <Layers3 className="text-foreground size-6" />
                <h3 className="text-2xl font-semibold">Changes</h3>
                <p className="text-muted-foreground text-base leading-relaxed">
                  Every style, text, DOM, and comment edit collected in
                  reverse-chronological order. Keep element groups or add
                  detected component/source headings, filter by kind, search by
                  value, and review or revert recorded edits. See the{" "}
                  <Link
                    href="/docs/changes-tab"
                    className="underline underline-offset-4"
                  >
                    Changes guide
                  </Link>{" "}
                  for storage, JSON export and sharing scope.
                </p>
                <ul className="text-muted-foreground mt-2 space-y-2 text-base">
                  <li>• Sticky search + filter chips header</li>
                  <li>• Numbered pin badges match the page overlay</li>
                  <li>• Elements / Components grouping selector</li>
                  <li>• Resolve / Reopen / Edit / Delete per row</li>
                  <li>• Export / Import as JSON for diff handoff</li>
                </ul>
              </CardContent>
            </Card>
          </div>
        </div>
      </section>

      {/* Bottom — yellow background slab */}
      <Background variant="bottom">
        <section className="py-20 lg:py-28">
          <DashedLine className="container max-w-5xl" />
          <div className="container mt-16 max-w-5xl">
            <div className="mb-8 flex items-baseline gap-4">
              <span className="text-muted-foreground font-mono text-base tracking-wide uppercase">
                Row 3
              </span>
              <h2 className="text-2xl tracking-tight md:text-3xl">
                Bottom — ship your edits
              </h2>
            </div>
            <p className="text-muted-foreground mb-10 max-w-2xl">
              Two primary hand-off buttons — one to the clipboard, one to a
              connected AI coding agent. JSON export and MCP reads are also
              sharing paths. Review the{" "}
              <Link href="/privacy" className="underline underline-offset-4">
                privacy and sharing boundaries
              </Link>{" "}
              before sending.
            </p>
            <div className="grid gap-4 md:grid-cols-2">
              <Card>
                <CardContent className="flex flex-col gap-2 p-6">
                  <div className="flex items-center gap-2">
                    <Copy className="text-foreground size-6" />
                    <h3 className="text-2xl font-semibold">Copy as Prompt</h3>
                  </div>
                  <p className="text-muted-foreground text-base leading-relaxed">
                    Copies your recorded changes as a Markdown prompt. In the
                    next-release source this can include all saved routes on the
                    current site, even collapsed groups. Review it before
                    pasting; no MCP setup is required. See the{" "}
                    <Link
                      href="/blog/turn-visual-edits-into-precise-ai-prompts"
                      className="underline underline-offset-4"
                    >
                      illustrative prompt and review checklist
                    </Link>
                    .
                  </p>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="flex flex-col gap-2 p-6">
                  <div className="flex items-center gap-2">
                    <Send className="text-foreground size-6" />
                    <h3 className="text-2xl font-semibold">Send to Agent</h3>
                  </div>
                  <p className="text-muted-foreground text-base leading-relaxed">
                    Marks recorded changes as ready over MCP. A connected agent
                    retrieves the live changes and hand-off marker through the
                    MCP tools. Without an attached agent, Design Mode opens
                    setup guidance instead of claiming the hand-off succeeded.
                    Local mode can continue into explicit feedback rounds with
                    immutable snapshots and a Stop action in the next-release
                    implementation. Follow the{" "}
                    <Link href="/mcp" className="underline underline-offset-4">
                      MCP setup guide
                    </Link>
                    ; the agent still needs authorised repository access and its
                    changes need review.
                  </p>
                </CardContent>
              </Card>
            </div>
          </div>
        </section>
      </Background>
    </>
  );
}
