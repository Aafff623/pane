import { mcpClients } from "@/lib/mcp-guide";

export type DocPage = {
  slug: string;
  title: string;
  metaTitle: string;
  metaDescription: string;
  keywords: string[];
  intro: string;
  sections: {
    heading: string;
    body: string;
    code?: string;
    links?: { href: string; label: string }[];
  }[];
  related: string[];
};

export const docs: DocPage[] = [
  {
    slug: "browser-support",
    title: "Browser support & parity",
    metaTitle: "Browser support — Chrome, Firefox, Safari and feature parity",
    metaDescription:
      "Design Mode runs on Chrome, Edge, Brave, Arc, Firefox and Safari on Mac. What's identical across browsers, and the few browser-specific features (including Safari's manual setup).",
    keywords: [
      "Design Mode browser support",
      "Design Mode Firefox",
      "Firefox add-on",
      "Chrome vs Firefox extension",
      "browser feature parity",
    ],
    intro:
      "Design Mode runs on Chromium browsers (Chrome, Edge, Brave, Arc), Firefox 121+ and Safari on Mac. Everything core is available across browsers; a few extras depend on browser APIs, and Safari needs a short manual setup.",
    sections: [
      {
        heading: "Supported browsers",
        body: "Chromium browsers with Manifest V3 side panels — Chrome, Edge, Brave, Arc — install from the Chrome Web Store and render the editor in the right-side panel. Firefox (121+) installs from Firefox Add-ons (AMO) and renders in the native sidebar. Safari on Mac installs manually from designmode-for-safari.zip on the latest GitHub release (https://github.com/SandeepBaskaran/design-mode/releases/latest); the setup takes under 30 seconds. It's a desktop-primary experience.",
      },
      {
        heading: "Identical on every browser",
        body: "The whole editing surface is the same: inspect any element; edit Position, Layout, Typography, Fill, Stroke, Effects, Motion, and Layout Guides; the Layers tree; the Changes tab with export/import; comments; element and viewport screenshots; DOM edits (duplicate, delete, reorder); the design-token engine; and the full MCP / send-to-agent handoff. Firefox always opens its sidebar; Chrome can launch into its side panel or Chrome-only floating surfaces.",
      },
      {
        heading: "Browser-specific features",
        body: "Three extras rely on Chromium-only browser APIs and are hidden on Firefox: the pop-out floating window (needs the side-panel + windows APIs), Picture-in-Picture “pin on top” (needs the Document Picture-in-Picture API), and the screen eyedropper “Pick” button (needs the EyeDropper API). On Firefox, colour entry still works via the HSV picker, the site-token list, and hex/RGB input — only the whole-screen sampler is unavailable. Safari has its own Web Inspector installation and integration details below.",
      },
      {
        heading: "Safari specifics",
        body: "Safari on Mac uses a temporary Web Inspector extension from the latest GitHub release. Open Safari Settings → Developer → Add Temporary Extension, choose designmode-for-safari.zip, and re-add it after Safari quits or the temporary extension expires. The main editing workflow is available in the Inspector; this build does not use the Chrome Web Store or Firefox Add-ons.",
      },
      {
        heading: "Firefox specifics",
        body: "Design Mode opens as Firefox's native sidebar rather than a right-docked panel; the toolbar button, the View → Sidebar menu, and Alt+D all toggle it. For local files, Firefox manages file:// access from about:addons (there's no per-extension “Allow access to file URLs” toggle like Chrome's).",
      },
    ],
    related: ["keyboard-shortcuts", "troubleshooting"],
  },
  {
    slug: "keyboard-shortcuts",
    title: "Keyboard shortcuts",
    metaTitle:
      "Keyboard shortcuts — Design Mode side panel shortcuts reference",
    metaDescription:
      "Every keyboard shortcut in Design Mode — inspector, comments, tabs, undo/redo, screenshot, export CSS — including the two browser-level commands you can rebind. Designed for fast iteration without leaving the keyboard.",
    keywords: [
      "Design Mode keyboard shortcuts",
      "design tool shortcuts",
      "Chrome extension shortcuts",
      "keyboard reference",
    ],
    intro:
      "Design Mode has 12 shortcuts that work directly on the page while the side panel is open, plus two browser-level commands for opening the panel and taking a screenshot — 14 in total.",
    sections: [
      {
        heading: "In-page shortcuts",
        body: "These fire on the page itself while the side panel is open: Alt+I toggles Inspect, Alt+C drops a comment pin, Alt+R starts a region comment (drag a rectangle), Alt+P pauses/resumes all motion on the page, and Alt+X copies the exported CSS to your clipboard.",
      },
      {
        heading: "Tabs & selection",
        body: "Alt+1 / Alt+2 / Alt+3 jump to the Layers / Design / Changes tab. Escape deselects back to hover mode. Delete removes the selected element.",
      },
      {
        heading: "Undo & redo",
        body: "Cmd/Ctrl + Z undoes the last style, text, or DOM change. Cmd/Ctrl + Shift + Z redoes it. Every change in the Changes tab is reversible individually too.",
      },
      {
        heading: "Browser-level commands",
        body: "Alt+D opens Design Mode on both Chrome and Firefox; Alt+S takes a screenshot using the same target and destination as the camera button. They are browser commands rather than page handlers, so both can be rebound at chrome://extensions/shortcuts on Chrome or about:addons › Manage Extension Shortcuts on Firefox. Browsers treat the supplied keys as suggestions and may leave either command unassigned after an update or conflict. The 12 in-page shortcuts above cannot be remapped from Settings — that list is read-only.",
      },
    ],
    related: ["changes-tab", "mcp-setup"],
  },
  {
    slug: "mcp-setup",
    title: "MCP setup",
    metaTitle: "MCP setup for AI apps — Cloud, Local and Self-hosted",
    metaDescription:
      "Connect Design Mode using anonymous bearer-token pairing, protected client configuration and read-only tests. Examples for Claude Code, Cursor and VS Code.",
    keywords: [
      "MCP setup",
      "Claude Code MCP",
      "Cursor MCP",
      "VS Code MCP",
      "Claude Desktop MCP",
    ],
    intro:
      "Start with the copyable setup prompt at https://designmode.app/mcp, or configure your client below. Examples were checked against first-party documentation on 29 September 2026. MCP support, policy and reload behaviour vary by client version.",
    sections: [
      {
        heading: "1. Cloud — the default",
        body: "Open the extension's MCP page from its header chip. Select Cloud and create an anonymous credential. The agent endpoint is https://mcp.designmode.app/mcp (Streamable HTTP). The extension and agent use the same bearer token; there is no Design Mode OAuth login, account check or subscription verification. Enter the token locally through a protected client input or securely supplied environment variable, never in AI chat, shell history, screenshots or committed configuration.",
      },
      {
        heading: "2. Local — run from source",
        body: 'Review and clone https://github.com/SandeepBaskaran/design-mode and run npm ci at the repository root. Configure your MCP client to launch the stdio command below with the absolute checkout path; then select Local in the extension. In JSON use command npm and args ["start", "--prefix", "/absolute/path/to/design-mode/packages/mcp-local"]. The agent CLI package is private; no published CLI installation is assumed. Clients share a bridge owner on localhost:9960. If another app owns that port, use DM_PORT and match the extension\'s Local port. Never kill a foreign process.',
        code: "npm start --prefix /absolute/path/to/design-mode/packages/mcp-local",
      },
      {
        heading: "3. Self-hosted — your relay",
        body: "Deploy packages/mcp-cloud on a Node.js host with Redis and TLS. Use the extension's Self-hosted mode with your relay's base URL, create a credential on that relay and configure the agent with https://your-relay.example/mcp. Replace the Cloud endpoint in the examples, and supply that relay's token—not the hosted Cloud credential. See https://github.com/SandeepBaskaran/design-mode/tree/main/packages/mcp-cloud for deployment prerequisites.",
      },
      ...mcpClients.map((client) => ({
        heading: client.name + " — " + client.location,
        body: client.description + " Official reference: " + client.source,
        code: client.code,
      })),
      {
        heading: "Other apps and incompatible connectors",
        body: "Cloud and Self-hosted need Streamable HTTP plus a custom Authorization header. An OAuth-only connector or a client without custom-header support cannot use this bearer token directly. For Claude Desktop, use the documented local stdio configuration via Settings → Developer → Edit Config, then fully quit and restart; see https://modelcontextprotocol.io/docs/develop/connect-local-servers. For Windsurf, Cline and other apps, inspect current first-party transport and credential support rather than reusing another client's JSON. If neither HTTP with headers nor local stdio works, use Copy as Prompt from the Changes tab.",
      },
      {
        heading: "Approve access and test safely",
        body: "Merge configuration without replacing unrelated servers. Review and trust the endpoint or local command, start or reload the server, and restart the app if needed. Discover tools and schemas, then call get_session_summary with {}. Confirm the actual browser/session state; an MCP chip alone is not proof. With permission to read the page, call get_changes with {}. Empty edits are a valid result. Do not modify a page, clear changes or resolve comments during this test. Keep writes separately approved and revoke an exposed credential in the extension's MCP page.",
        code: "get_session_summary({})\nget_changes({})",
      },
      {
        heading: "Available tools and progressive reads",
        body: "All modes expose get_session_summary, get_changes, get_screenshot, export_changes, apply_changes, set_change_status, mark_comment_resolved and clear_changes. Start with the summary, then fetch changes once and select unresolved IDs locally: get_changes has no pagination, status filter or max_chars. Use get_screenshot with a returned selector, elementId or commentId (not a raw region argument). export_changes requires format css, tailwind, scss or jsx. Bound writes to explicit IDs; omitting ids in set_change_status affects all items. The icon-led reference is at https://designmode.app/mcp.",
      },
      {
        heading: "Local live rounds and the agent skill",
        body: "Only Local exposes wait_for_handoff. After explicit opt-in, start with exact pageUrl and timeoutMs up to 20000; resume using the returned sessionId and after cursor. Each feedback report is an immutable Send snapshot. Stop on stopped, busy or an error. Cloud and Self-hosted use one-shot reads, not automatic background polling. Download the canonical skill at https://designmode.app/.well-known/agent-skills/design-mode/SKILL.md; its index is https://designmode.app/.well-known/agent-skills/index.json. A skill does not grant permissions or configure a connection by itself.",
      },
    ],
    related: ["troubleshooting", "changes-tab"],
  },
  {
    slug: "changes-tab",
    title: "The Changes tab",
    metaTitle: "Changes tab — Searchable, exportable design change history",
    metaDescription:
      "Every edit in Design Mode lands in the Changes tab — style, text, DOM, comments. Search, filter, resolve, export or import JSON, and copy a Markdown prompt.",
    keywords: [
      "Design Mode Changes tab",
      "design change history",
      "structured design diff",
      "design export",
    ],
    intro:
      "The Changes tab is where every edit you make in the side panel lands. It's the foundation of the handoff — to a teammate, to a tracker, to an AI agent.",
    sections: [
      {
        heading: "What lands in the Changes tab",
        body: "Style changes (typography, colour, spacing, layout, motion, effects), text changes, DOM mutations (duplicate, delete, reorder, restructure), and comment pins. Each row records the selector, property, old value, and new value.",
      },
      {
        heading: "Search, filter, group",
        body: "Sticky header with search and filter chips at the top. Search by selector, component/source hint, property name, or value. Filter by kind (style / text / DOM / comment). Keep the default element groups or add detected React/Vue component and source headings; selectors and per-element actions remain available, and missing metadata stays explicitly unattributed.",
      },
      {
        heading: "Resolve, reopen, edit, delete",
        body: "Each row has per-item actions. Resolved items collapse but remain in history.",
      },
      {
        heading: "Export & import",
        body: "Export downloads the recorded changes as JSON. Import accepts JSON and replaces the current changes, so export a backup first. For a readable Markdown hand-off to a teammate, issue or agent, use Copy as Prompt; Markdown is not an import format.",
      },
    ],
    related: ["mcp-setup", "troubleshooting"],
  },
  {
    slug: "troubleshooting",
    title: "Troubleshooting",
    metaTitle:
      "Troubleshooting Design Mode — side panel won't open, MCP not connecting",
    metaDescription:
      "Fix common Design Mode issues: side panel won't open, MCP not connecting, edits not persisting, the Send-to-Agent button greyed out.",
    keywords: [
      "Design Mode troubleshooting",
      "Design Mode not working",
      "MCP not connecting",
      "Chrome extension fix",
      "side panel won't open",
    ],
    intro:
      "Most issues fall into a handful of patterns. Here's the short list.",
    sections: [
      {
        heading: "The side panel doesn't open",
        body: "Update to a current Chrome, Edge, Brave or Arc release that supports Manifest V3 side panels. The extension manifest does not declare an exact Chrome minimum. In Chrome, check Settings → Launch: Floating and Pin on top open a separate window rather than the docked panel. Some enterprise policies block the side panel API; try a clean Chrome profile to rule out a conflicting extension.",
      },
      {
        heading: "MCP status chip stays offline",
        body: "Cloud mode: confirm your bearer token is pasted on the extension's dedicated MCP page (opened from the header MCP chip) and the same token is in your agent's config. Local mode: confirm the companion server is running (clone the repo, npm install, npm start) and your config's cwd points at the repo root. Multiple Design Mode clients can share port 9960; never add a kill -9 startup command. If another application owns 9960, choose a different DM_PORT such as 9961 and set the extension's Local port to match. Self-hosted: confirm the relay URL is correct and Redis is healthy.",
      },
      {
        heading: "Send to Agent button is greyed out",
        body: "The button only enables when an agent is actually attached (the MCP status chip will be the connected colour). Restart your agent after pasting the config snippet — most clients only re-read MCP servers at startup.",
      },
      {
        heading: "Edits aren't persisting",
        body: "Chromium stores each per-URL change session in storage.session, so it survives reloads but not a full browser restart. Firefox falls back to storage.local. Incognito windows and separate browser profiles have separate extension storage. If the Changes tab still lists an edit after reload, the site may simply have re-rendered over the browser preview.",
      },
      {
        heading: "Still stuck?",
        body: "Open the Help panel inside the extension (? icon) → Copy diagnostics → file an issue on GitHub. The diagnostics block has the environment metadata maintainers need.",
      },
    ],
    related: ["mcp-setup", "changes-tab"],
  },
];

export function getDoc(slug: string): DocPage | undefined {
  return docs.find((d) => d.slug === slug);
}
