export type ComparisonRow = {
  feature: string;
  designMode: string;
  competitor: string;
};

export type Comparison = {
  slug: string;
  competitor: string;
  title: string;
  metaTitle: string;
  metaDescription: string;
  keywords: string[];
  oneLiner: string;
  positioning: string;
  whenToPickDesignMode: string[];
  whenToPickCompetitor: string[];
  table: ComparisonRow[];
  honesty: string;
  research?: {
    checkedOn: string;
    methodology: string;
    sources: { label: string; url: string }[];
  };
  related: string[];
};

export function isComparisonIndexable(comparison: Comparison): boolean {
  return Boolean(
    comparison.research?.checkedOn &&
    comparison.research.methodology &&
    comparison.research.sources.length,
  );
}

const baseFeatures = (
  competitorBlanks: Record<string, string>,
): ComparisonRow[] => [
  {
    feature: "Visual editing on a scriptable webpage",
    designMode:
      "Yes — typography, colour, layout, spacing, motion and effects on localhost, staging or production pages the extension can script. Browser-protected pages are excluded.",
    competitor: competitorBlanks.editing ?? "Not yet verified",
  },
  {
    feature: "MCP (Model Context Protocol) handoff to AI agents",
    designMode:
      "Yes — Cloud, Local, and Self-hosted modes; eight common session tools, plus Local feedback rounds",
    competitor: competitorBlanks.mcp ?? "Not yet verified",
  },
  {
    feature: "Persistent change history (Changes tab)",
    designMode: "Yes — searchable, filterable, exportable",
    competitor: competitorBlanks.history ?? "Not yet verified",
  },
  {
    feature: "Open source",
    designMode: "Yes (MIT)",
    competitor: competitorBlanks.os ?? "Not yet verified",
  },
  {
    feature: "Price",
    designMode: "Free MIT-licensed extension",
    competitor: competitorBlanks.price ?? "Not yet verified",
  },
  {
    feature: "Markdown / JSON export of the diff",
    designMode: "Yes",
    competitor: competitorBlanks.export ?? "Not yet verified",
  },
  {
    feature: "Best fit for",
    designMode:
      "Designers, developers, QA, PMs, content, indie hackers, agencies, vibe coders",
    competitor: competitorBlanks.fit ?? "Not yet verified",
  },
];

export const comparisons: Comparison[] = [
  {
    slug: "design-mode-vs-magicpath",
    competitor: "MagicPath",
    title: "Design Mode vs MagicPath",
    metaTitle: "Design Mode vs MagicPath — browser edits or a shared AI canvas",
    metaDescription:
      "Compare Design Mode's live-page edits and agent hand-off with MagicPath's shared cloud canvas, interactive prototypes, external agents and credit-based plans.",
    keywords: [
      "Design Mode vs MagicPath",
      "MagicPath alternative",
      "AI design canvas",
    ],
    oneLiner:
      "Design Mode refines an existing webpage and records changes for your coding agent. MagicPath provides a shared cloud canvas for creating interactive designs with humans and agents.",
    positioning:
      "Choose by starting point: a rendered product you want to refine, or a canvas where you want to create and collaborate. MagicPath 2.0 positions itself as a shared workspace for interactive prototypes and apps, with external-agent access and code export. Design Mode keeps the visual review on your existing page and leaves source implementation to your agent. Neither workflow removes the need to review and test the resulting code.",
    whenToPickDesignMode: [
      "You want to refine localhost, staging or production without moving the project into another design workspace.",
      "You want a free MIT-licensed editor for Chrome, Firefox or Safari with portable prompt exports and optional MCP hand-off.",
      "Your existing repository, coding agent and release process should remain the source of truth.",
    ],
    whenToPickCompetitor: [
      "You want to create interactive prototypes and explore multiple designs in a shared canvas.",
      "Your team wants a cloud workspace that external agents can access even when the canvas is closed.",
      "Figma import/export, component libraries and team administration matter more than editing an existing page in place.",
    ],
    table: [
      {
        feature: "Starting point",
        designMode:
          "A scriptable webpage rendered in Chrome, Firefox or Safari. Browser-protected pages are excluded.",
        competitor:
          "A shared visual canvas, accessible through its web app or macOS app, for interactive prototypes and app creation.",
      },
      {
        feature: "Source-code workflow",
        designMode:
          "Records browser previews and a structured specification. Your agent needs separate repository access to implement it.",
        competitor:
          "Advertises runnable prototypes, code downloads and external-agent workflows between the canvas and a repository. Source round-trip fidelity was not tested.",
      },
      {
        feature: "Agents and collaboration",
        designMode:
          "Copy as Prompt or Cloud, Local and Self-hosted MCP hand-off from the browser session.",
        competitor:
          "Cloud canvas for humans and agents. Lists Claude Code, Codex and Cursor; free accounts have 50 external agent calls per week, paid plans list unlimited calls.",
      },
      {
        feature: "Design assets",
        designMode:
          "Inspect and refine the existing page's layout, typography, colours and other visual properties in context.",
        competitor:
          "Lists component libraries, custom fonts, use of your own design system and plan-dependent Figma import/export allowances. Its pricing page also lists a Chrome extension; that extension was not evaluated.",
      },
      {
        feature: "Cost and allowances",
        designMode:
          "Free MIT-licensed extension. Your coding agent or model provider may charge separately.",
        competitor:
          "Free: 20 generation credits/day, capped at 120/month. Annual billing shown: Builder $84/year ($7/month equivalent); Pro $252/year ($21/month equivalent) with the displayed 600-credit pack. Teams: custom. External-agent billing is separate from MagicPath credits. Checked 16 September 2026.",
      },
    ],
    honesty:
      "MagicPath is the broader creation and collaboration workspace; Design Mode is the focused browser-editing and hand-off tool. MagicPath's credits fund its own AI generation, not external-agent usage, and its pricing FAQ says manual edits, exports and library use do not spend credits. Annual equivalents are not month-to-month prices. Validate code export, repository integration and collaboration with a representative project before committing.",
    research: {
      checkedOn: "16 September 2026",
      methodology:
        "Documentation review of MagicPath's current 2.0 homepage and pricing page, including plan allowances and billing FAQs. No account, installation, paid plan, canvas editing or agent integration was tested. Product capabilities are vendor-documented, not independently runtime-verified.",
      sources: [
        { label: "MagicPath 2.0 product", url: "https://www.magicpath.ai/" },
        {
          label: "MagicPath pricing and billing FAQ",
          url: "https://www.magicpath.ai/pricing",
        },
      ],
    },
    related: [
      "design-mode-vs-backdraft",
      "design-mode-vs-uiprompt",
      "design-mode-vs-css-studio",
    ],
  },
  {
    slug: "design-mode-vs-backdraft",
    competitor: "Backdraft",
    title: "Design Mode vs Backdraft",
    metaTitle: "Design Mode vs Backdraft — browser hand-off or source editing",
    metaDescription:
      "Compare Design Mode's free browser editing and agent hand-off with Backdraft's paid code-and-canvas workspace, responsive previews and source editing.",
    keywords: [
      "Design Mode vs Backdraft",
      "Backdraft alternative",
      "visual source code editor",
    ],
    oneLiner:
      "Design Mode previews changes on a live page and hands them to your agent. Backdraft brings source files, a visual canvas and coding agents into one paid workspace.",
    positioning:
      "The main difference is who owns the source-editing step. Design Mode runs on a scriptable webpage, records your visual changes and lets your existing coding agent implement them. Backdraft's documented workflow imports a project into a code-and-canvas editor, where visual edits write back to source. It is a broader development environment, not just a browser feedback tool.",
    whenToPickDesignMode: [
      "You want to review localhost, staging or production without importing a project into another editor.",
      "You prefer to keep source edits, Git and deployment in your existing development tools.",
      "You need free, MIT-licensed visual editing, exports and optional MCP hand-off.",
    ],
    whenToPickCompetitor: [
      "You want a visual canvas and code editor working on the same source files.",
      "You want to evaluate its multi-page, multi-breakpoint Overwatch view and integrated screenshot-verification tools.",
      "You want project import and deployment tools in the same workspace and accept the relevant paid plan.",
    ],
    table: [
      {
        feature: "Starting point",
        designMode:
          "A rendered webpage in Chrome, Firefox or Safari; no project import or site-code changes needed.",
        competitor:
          "An imported HTML/CSS, React/Tailwind or supported TypeScript project in a desktop or web canvas.",
      },
      {
        feature: "What an edit changes",
        designMode:
          "The browser preview and its recorded specification. An agent or developer must implement the source change.",
        competitor:
          "According to its documentation, visual edits write to source and code edits update the canvas. Source fidelity was not hands-on tested.",
      },
      {
        feature: "Responsive review",
        designMode:
          "Responsive preview and breakpoint-tagged changes on the page being inspected.",
        competitor:
          "Overwatch displays multiple pages at desktop, tablet and mobile sizes; agent tools include screenshots and visual comparison.",
      },
      {
        feature: "Agent and delivery workflow",
        designMode:
          "Copy/export a specification or use Cloud, Local or Self-hosted MCP. Keep your existing agent, repository and release process.",
        competitor:
          "Built-in agent workflow plus CLI-agent integrations and GitHub tools. Figma/Webflow imports and Netlify deployment are listed under Pro.",
      },
      {
        feature: "Tool price",
        designMode:
          "Free under MIT. Paid agent or model usage remains separate.",
        competitor:
          "Basic: $9/month or $84/year. Pro: $19/month or $180/year. Seven-day trial advertised. Model/API costs are separate; prices checked 16 September 2026.",
      },
    ],
    honesty:
      "Backdraft is worth evaluating if you want to move more of development into a visual workspace. Design Mode is the lighter choice when you want precise browser feedback without replacing your editor or deployment process. Backdraft also advertises a $135 one-year desktop licence available to Pro users: it is not a lifetime purchase. Validate its source round-trip behaviour on a disposable project before trusting it with your app.",
    research: {
      checkedOn: "16 September 2026",
      methodology:
        "Browsed the official site and exercised the monthly/annual pricing toggle; reviewed documentation and roadmap. The web app failed TLS certificate validation in the audit browser, so its editor, imports and source writes were not tested. No certificate bypass, installation or trial was attempted. Platform and roadmap inconsistencies are not treated as verified capabilities.",
      sources: [
        { label: "Backdraft product", url: "https://backdraftai.com/" },
        {
          label: "Backdraft pricing",
          url: "https://backdraftai.com/pricing.html",
        },
        {
          label: "Backdraft documentation",
          url: "https://backdraftai.com/docs.html",
        },
        {
          label: "Backdraft roadmap",
          url: "https://backdraftai.com/roadmap.html",
        },
      ],
    },
    related: [
      "design-mode-vs-css-studio",
      "design-mode-vs-handle-extension",
      "design-mode-vs-uiprompt",
    ],
  },
  {
    slug: "design-mode-vs-uiprompt",
    competitor: "UIPrompt",
    title: "Design Mode vs UIPrompt",
    metaTitle: "Design Mode vs UIPrompt — live-page edits or component specs",
    metaDescription:
      "Compare Design Mode's live-page editing with UIPrompt's component specifications, interaction-state diffs, theme tokens and prompt exports. Includes free and paid limits.",
    keywords: [
      "Design Mode vs UIPrompt",
      "UIPrompt alternative",
      "component design prompts",
    ],
    oneLiner:
      "Design Mode starts with your rendered product. UIPrompt starts with a component design and turns its values, states and tokens into an implementation prompt.",
    positioning:
      "These tools solve different stages of the same problem: communicating precise design intent to a coding agent. Design Mode captures changes in the context of an existing page. UIPrompt offers a standalone component playground with exact values, interaction-state overrides and theme-aware tokens. It specifies what to build rather than editing your deployed interface.",
    whenToPickDesignMode: [
      "The component already exists in your product and you need to refine it alongside real layout and content.",
      "You need page-level edits, element or region comments, and optional direct MCP hand-off.",
      "You want free editing and exports without saved-component limits.",
    ],
    whenToPickCompetitor: [
      "You are defining a reusable component before it exists in the app.",
      "You want default, hover, focus, active and disabled specifications with explicit differences between states.",
      "You want a shared-token component-library prompt rather than a diff from a live webpage.",
    ],
    table: [
      {
        feature: "Design surface",
        designMode:
          "Edit the real rendered page, including styles, text and DOM structure on supported scriptable websites.",
        competitor:
          "Standalone browser component editor. Its public catalogue lists twelve component types, including buttons, inputs, tabs and toasts.",
      },
      {
        feature: "Output for the agent",
        designMode:
          "Recorded changes and comments as a copied prompt, export or MCP hand-off. The agent implements them in your repository.",
        competitor:
          "Generated component or system prompts. The public editor exposes Claude Code, Cursor and v0 targets, React/CSS/Vue output options, and full or concise text.",
      },
      {
        feature: "States and design tokens",
        designMode:
          "Detect and edit existing page tokens, preserve token references in exports, and record trigger-specific motion changes. Starts from the page rather than a standalone component library.",
        competitor:
          "Explicit state overrides and tokens with light/dark values. In the playground, a hover-colour edit produced a differences-only section in the prompt.",
      },
      {
        feature: "Implementation guidance",
        designMode:
          "Send the intended change with page context; review the agent's implementation and test accessibility in the real app.",
        competitor:
          "Generated prompts include focus-visible, reduced-motion, contrast and hit-target guidance. These instructions do not prove the resulting component is accessible.",
      },
      {
        feature: "Cost and saving",
        designMode:
          "Free under MIT; no paid component quota. Any paid coding agent or model is separate.",
        competitor:
          "Anonymous playground; free accounts save three components with full single-component prompts. Advertised $39 one-time launch price (regular $59) unlocks unlimited components and system-pack export. Agent costs are separate.",
      },
    ],
    honesty:
      "UIPrompt is a useful specification tool, not a direct substitute for live-page editing. Its strongest idea is making interaction states and implementation constraints explicit. Use it to define a component library; use Design Mode to refine how components actually behave in a product. The vendor's exact-match experiment is not a guarantee that every agent will reproduce every design correctly.",
    research: {
      checkedOn: "16 September 2026",
      methodology:
        "Opened the anonymous editor, changed a button radius from 8px to 12px and confirmed the prompt updated. Selected Hover, changed its background to #2563EB and confirmed a separate hover-diff section. Inspected state, token and output controls. Pricing and paid system-pack/share capabilities come from the homepage; no login, purchase, public sharing or paid export was tested. No editor version was exposed.",
      sources: [
        {
          label: "UIPrompt product and pricing FAQ",
          url: "https://uiprompt.co/",
        },
        { label: "UIPrompt public editor", url: "https://uiprompt.co/editor" },
      ],
    },
    related: [
      "design-mode-vs-css-studio",
      "design-mode-vs-backdraft",
      "design-mode-vs-figma-dev-mode",
    ],
  },
  {
    slug: "design-mode-vs-handle-extension",
    competitor: "Handle Extension",
    title: "Design Mode vs Handle Extension",
    metaTitle:
      "Design Mode vs Handle Extension — two free browser editing tools",
    metaDescription:
      "Compare Design Mode and Handle Extension: two free MIT-licensed visual browser tools with different agent setup, connection and feedback workflows.",
    keywords: [
      "Design Mode vs Handle",
      "Handle Extension alternative",
      "handle-ext",
      "browser editing for AI agents",
    ],
    oneLiner:
      "Both are free, MIT-licensed browser tools for visual changes and agent hand-off. The choice is between Design Mode's exports and connection modes, and Handle's local agent-session workflow.",
    positioning:
      "Handle Extension is a close alternative, not just an annotation tool. Its Chrome side panel supports visual refinements, inline text editing and natural-language feedback, then returns context to your coding agent. Design Mode offers live editing, recorded changes and comments with copy/export hand-off or Cloud, Local and Self-hosted MCP. The meaningful difference is workflow, not a free-versus-paid split.",
    whenToPickDesignMode: [
      "You use Firefox as well as Chromium, or want to review a scriptable page without first starting an agent session.",
      "You want a portable copied/exported specification as well as direct MCP hand-off.",
      "You want to choose between hosted Cloud, Local and Self-hosted relay connections.",
    ],
    whenToPickCompetitor: [
      "You work in Chrome with a local dev server and want the agent to initiate a live feedback session.",
      "You want its setup command to configure a supported agent rather than enter the MCP configuration yourself.",
      "You want to evaluate its session discovery and repeated feedback rounds with an explicit stop action.",
    ],
    table: [
      {
        feature: "Core editing workflow",
        designMode:
          "Edit the rendered page, record changes, add comment pins and hand the specification to a developer or agent.",
        competitor:
          "Select elements in a Chrome side panel, refine styles or text, attach notes and send structured feedback to the active agent.",
      },
      {
        feature: "Setup and browsers",
        designMode:
          "Chrome/Chromium or Firefox extension. Start editing without MCP; configure it when you want a connected agent.",
        competitor:
          "Chrome extension plus a local MCP server. The documented setup runs npx handle-ext@latest init and restarts the agent; invocation varies by agent.",
      },
      {
        feature: "Connection model",
        designMode:
          "Cloud, Local or Self-hosted MCP, plus copy/export without a live agent connection.",
        competitor:
          "Documented local stdio MCP and Socket.IO bridge. A discovery server exposes active sessions with agent and repository context.",
      },
      {
        feature: "Feedback rounds",
        designMode:
          "Agent tools read changes, return browser previews and update statuses. Local mode also supports opt-in feedback rounds with immutable per-Send snapshots and an explicit Stop; Cloud and Self-hosted remain one-shot.",
        competitor:
          "The published MCP implementation continues a live feedback session across rounds, with a user-stop action and a grace period for brief extension disconnects.",
      },
      {
        feature: "Cost and licence",
        designMode:
          "Free, MIT-licensed. Your paid agent or model costs are separate.",
        competitor:
          "Also free and MIT-licensed, using your existing agent. No paid extension tier is advertised on the reviewed page; agent costs remain separate.",
      },
    ],
    honesty:
      "Handle deserves consideration if your work already revolves around a local coding-agent session. Design Mode is the better fit when browser choice, portable hand-off or relay choice matters more. Do not confuse Handle Extension with Handle Studio: the separate Studio app's macOS requirement is not an extension restriction. Neither browser tool independently guarantees a correct source-code change.",
    research: {
      checkedOn: "16 September 2026",
      methodology:
        "Browsed the extension page and its v1.0.3 revision notes; reviewed the public repository, MIT licence and MCP server source. The repository documents agent-specific commands rather than a universal /handle invocation. No extension installation, setup command or live agent round trip was performed; implementation details are source-reviewed, not runtime-verified.",
      sources: [
        {
          label: "Handle Extension product and revision notes",
          url: "https://gethandle.ai/extension",
        },
        {
          label: "Handle setup and architecture",
          url: "https://github.com/tonkotsu-ai/handle",
        },
        {
          label: "Handle MIT licence",
          url: "https://github.com/tonkotsu-ai/handle/blob/main/LICENSE",
        },
        {
          label: "Handle agent-session implementation",
          url: "https://github.com/tonkotsu-ai/handle/blob/main/mcp/src/server.ts",
        },
      ],
    },
    related: [
      "design-mode-vs-css-studio",
      "design-mode-vs-ui-ticket-mcp",
      "design-mode-vs-stagewise",
    ],
  },
  {
    slug: "design-mode-vs-ui-ticket-mcp",
    competitor: "UI Ticket MCP",
    title: "Design Mode vs UI Ticket MCP",
    metaTitle: "Design Mode vs UI Ticket MCP — visual edits or review tickets",
    metaDescription:
      "Compare Design Mode and UI Ticket MCP by visual editing, review threads, setup, storage and commercial-use licensing. Choose the workflow that fits your review.",
    keywords: [
      "Design Mode vs UI Ticket MCP",
      "UI Ticket MCP alternative",
      "ui-ticket-mcp",
      "visual feedback for AI coding agents",
    ],
    oneLiner:
      "Design Mode lets you make the visual change and hand off the specification. UI Ticket MCP turns annotated feedback into a review queue for your coding agent.",
    positioning:
      "Both connect browser feedback to an AI coding agent, but they start with different jobs. Design Mode combines live style, text and DOM editing with comment pins and structured change exports. UI Ticket MCP centres on tickets: point at an element, describe the problem, discuss it in a thread and let an agent read, implement and resolve the feedback. Choose based on whether you need to demonstrate the intended result or manage a conversation about what should change.",
    whenToPickDesignMode: [
      "You want to set the spacing, typography or copy yourself rather than describe the intended result in a ticket.",
      "You review scriptable localhost, staging or production pages without adding a review component to each app.",
      "You need an MIT-licensed tool for commercial work, with exports or optional MCP hand-off.",
    ],
    whenToPickCompetitor: [
      "Your primary job is reviewing prototypes through tagged tickets, threaded replies and an open-work queue.",
      "You control the app and want reviews stored with the project, or want to evaluate its optional multi-project Ticket Hub.",
      "Your use fits its non-commercial licence, or you have separately obtained the permissions needed for commercial use.",
    ],
    table: [
      {
        feature: "Core workflow",
        designMode:
          "Edit the rendered page, record exact changes and add element or region comments. Export the specification or send it to an agent.",
        competitor:
          "Annotate elements or regions, write feedback and let the agent implement it. The documented product centres on reviews rather than direct visual style editing.",
      },
      {
        feature: "Installation and page access",
        designMode:
          "Chrome, Firefox or Safari extension; no code added to the inspected app. Works on scriptable pages, not browser-protected surfaces.",
        competitor:
          "Embed a Web Component using npm or a CDN script. The local setup also runs a Python MCP server and HTTP API; SSR apps need client-side loading.",
      },
      {
        feature: "Review conversations",
        designMode:
          "Element and region comment pins, searchable Changes tab, and resolve or reopen through MCP.",
        competitor:
          "Threaded replies, bug/suggestion/question/general tags, search, open/resolved filters and per-page pending-work summaries.",
      },
      {
        feature: "Agent hand-off",
        designMode:
          "Copy or export changes without MCP, or connect through Cloud, Local or Self-hosted MCP. The agent still needs access to your source repository.",
        competitor:
          "MCP tools expose review context, pending work and resolution actions. A page-name-based source-file finder suggests matching files in the configured project.",
      },
      {
        feature: "Where feedback lives",
        designMode:
          "Comments are saved in local browser extension storage. Change exports provide a portable hand-off; the Cloud relay is not a shared ticket dashboard.",
        competitor:
          "Local reviews live in a project SQLite database. The v1.5.0 changelog also documents an optional multi-project Ticket Hub with a dashboard and Docker self-hosting.",
      },
      {
        feature: "Cost and commercial use",
        designMode:
          "Free under MIT, including commercial use subject to the licence terms. Paid agent or model usage is separate.",
        competitor:
          "Published under CC BY-NC 4.0, described as free for study, research and non-commercial use. Commercial-use permission and Hub pricing were not established in the reviewed sources; check with the maintainers. Agent costs are separate.",
      },
    ],
    honesty:
      "UI Ticket MCP has a stronger documented ticket-discussion workflow: tags, reply chains and a project-level queue are useful when feedback needs clarification before implementation. Design Mode is the better fit when you want to show the exact visual change, work without modifying the app, or use a permissively licensed tool commercially. Both can mark feedback resolved; neither status proves the resulting code is correct. Review the source diff and rendered result before accepting a fix.",
    research: {
      checkedOn: "14 September 2026",
      methodology:
        "Documentation comparison of the official site, public README, licence and changelog through v1.6.0, checked against Design Mode's implementation. UI Ticket MCP was not installed or hands-on tested. The site lists 11 MCP tools while the README lists 10; tool count is not used as a differentiator. Hub support comes from the newer changelog, not the README's local-only setup description.",
      sources: [
        {
          label: "UI Ticket MCP official site",
          url: "https://uiticket.0ics.ai/",
        },
        {
          label: "UI Ticket MCP setup and review workflow",
          url: "https://github.com/0ics-srls/ui-ticket-mcp_public#readme",
        },
        {
          label: "UI Ticket MCP licence",
          url: "https://github.com/0ics-srls/ui-ticket-mcp_public/blob/main/LICENSE",
        },
        {
          label: "UI Ticket MCP changelog and Ticket Hub",
          url: "https://github.com/0ics-srls/ui-ticket-mcp_public/blob/main/CHANGELOG.md",
        },
      ],
    },
    related: [
      "design-mode-vs-drawbridge",
      "design-mode-vs-pls-fix",
      "design-mode-vs-css-studio",
    ],
  },
  {
    slug: "design-mode-vs-css-studio",
    competitor: "CSS Studio",
    title: "Design Mode vs CSS Studio",
    metaTitle: "Design Mode vs CSS Studio — workflow and cost compared",
    metaDescription:
      "Compare Design Mode's free browser extension with CSS Studio's project-installed visual editor: live-page reviews, animation workflows, AI hand-off and cost.",
    keywords: [
      "Design Mode vs CSS Studio",
      "CSS Studio alternative",
      "CSS Studio pricing",
      "visual CSS editor",
    ],
    oneLiner:
      "Design Mode is a free extension for live-page editing and hand-off. CSS Studio installs into your site, with a free visual editor and a $99 one-time AI integration.",
    positioning:
      "Both let you make visual changes on a rendered website and ask your existing coding agent to implement them. The useful distinction is where you work: Design Mode suits reviews across localhost, staging and production without adding code to each site. CSS Studio's documented setup adds an editor to your own site in development mode, bringing visual controls, chat and animation authoring into that project.",
    whenToPickDesignMode: [
      "You review multiple live sites or staging builds and do not want to install an editor into each project.",
      "You want to inspect recorded changes and export a specification for a developer, or connect an MCP agent without buying a tool licence.",
      "Your work centres on design QA, copy changes and visual feedback on an existing page.",
    ],
    whenToPickCompetitor: [
      "You control the site's development setup and want visual editing, agent chat and tasks inside that project.",
      "Your work calls for CSS animation authoring with a scrubbable timeline, draggable keyframes, springs or scroll-linked motion.",
      "You prefer its integrated workflow and accept a one-time purchase for AI integration.",
    ],
    table: [
      {
        feature: "Where you work and what you install",
        designMode:
          "Chrome, Firefox or Safari extension on scriptable localhost, staging or production pages. No package or script added to the inspected site; browser-protected pages are excluded.",
        competitor:
          "Editor added to your site through the cssstudio package or a script tag. The installation guide runs it in development mode.",
      },
      {
        feature: "Best-fit use case",
        designMode:
          "Review and refine existing pages, record precise visual feedback, then hand it to a developer or coding agent.",
        competitor:
          "Iterate inside your own development project with visual editing, agent chat, generated variants and animation authoring.",
      },
      {
        feature: "Animation workflow",
        designMode:
          "Adjust motion through the side panel's animation and transition controls.",
        competitor:
          "Timeline editor with playback scrubbing, draggable CSS keyframes, spring easing and scroll-linked animations.",
      },
      {
        feature: "Getting edits into source code",
        designMode:
          "Review the Changes tab, copy or export a specification, or hand it to a connected MCP agent. The agent needs repository access to implement it.",
        competitor:
          "Connect Cursor directly, or configure MCP and the /studio skill for other agents. The agent implements visual edits in your source files; a copy-prompt fallback is documented.",
      },
      {
        feature: "Tool cost",
        designMode: "Free, MIT-licensed editor and MCP integration.",
        competitor:
          "Visual editor free without AI source updates. AI integration is a $99 one-time purchase; future core updates included. Pricing checked 14 September 2026.",
      },
      {
        feature: "AI costs beyond the tool",
        designMode:
          "Bring your own agent. Any paid agent subscription or model usage is separate from Design Mode.",
        competitor:
          "Bring your own agent. The one-time purchase does not include paid agent or model usage.",
      },
    ],
    honesty:
      "CSS Studio is a credible choice for project-local editing, especially when timeline-based animation work matters. Design Mode is the simpler fit for reviewing pages without modifying their setup and for free structured hand-off. Neither tool removes the need to review the agent's source diff and test the result: Design Mode's Changes tab records browser edits, not a guarantee that generated code is correct.",
    research: {
      checkedOn: "14 September 2026",
      methodology:
        "Documentation comparison using CSS Studio's first-party pricing and guides, and Design Mode's documented workflow. CSS Studio was not installed or hands-on tested; no tested version is claimed. Pricing and capabilities may change.",
      sources: [
        { label: "CSS Studio pricing", url: "https://cssstudio.ai/pricing" },
        {
          label: "CSS Studio installation and agent workflow",
          url: "https://cssstudio.ai/learn",
        },
        {
          label: "CSS Studio animation guide",
          url: "https://cssstudio.ai/learn/animations",
        },
      ],
    },
    related: [
      "design-mode-vs-stagewise",
      "design-mode-vs-cursor-design-mode",
      "design-mode-vs-visbug",
    ],
  },
  {
    slug: "design-mode-vs-stagewise",
    competitor: "Stagewise",
    title: "Design Mode vs Stagewise",
    metaTitle: "Design Mode vs Stagewise — open-source visual editor with MCP",
    metaDescription:
      "Honest comparison: Design Mode and Stagewise both ship visual edits to AI coding agents. Design Mode is MIT-licensed with a hosted Cloud relay, broader design controls, and built-in change history.",
    keywords: [
      "Design Mode vs Stagewise",
      "Stagewise alternative",
      "Stagewise comparison",
      "visual editor for AI agents",
      "MCP design tool",
    ],
    oneLiner:
      "Stagewise is the closest direct competitor — both target the AI-coding-agent design loop.",
    positioning:
      "Both tools target the same pain point: getting design intent from a live page into your AI coding agent. The differences come down to licensing, hosting, design control depth, and how change history is handled.",
    whenToPickDesignMode: [
      "You want MIT open source with no licensing risk.",
      "You don't want to run your own relay — Cloud mode is hosted at mcp.designmode.app.",
      "You need a broader design control set (motion, effects, variants, contrast checker).",
      "You want persistent, searchable change history in a Changes tab.",
      "You want use-case-specific workflows beyond developer-focused editing (UI testing, copy edits, design system audits).",
    ],
    whenToPickCompetitor: [
      "You're already deep in Stagewise's ecosystem and tooling.",
      "Stagewise's specific UX matches your team's mental model better.",
    ],
    table: baseFeatures({
      editing: "Yes — developer-focused visual editing",
      mcp: "Yes",
      history: "Limited",
      os: "Check current licence",
      price: "Check current pricing",
      export: "Partial",
      fit: "Developers using AI coding agents",
    }),
    honesty:
      "Stagewise is genuinely good and was earlier to this space. If you're happy with it, stay with it. Design Mode's case is breadth: it covers personas (QA, PMs, content, design system maintenance) Stagewise doesn't centre.",
    related: [
      "design-mode-vs-cursor-design-mode",
      "design-mode-vs-pls-fix",
      "design-mode-vs-drawbridge",
    ],
  },
  {
    slug: "design-mode-vs-pls-fix",
    competitor: "pls-fix",
    title: "Design Mode vs pls-fix",
    metaTitle: "Design Mode vs pls-fix — visual editor vs comment-pin bug tool",
    metaDescription:
      "Comparison: pls-fix is a comment-pin tool for filing visual bugs. Design Mode covers comment pins plus full visual editing, MCP handoff, persistent change history, and exports.",
    keywords: [
      "Design Mode vs pls-fix",
      "pls-fix alternative",
      "visual bug report tool",
      "comment pin tool",
      "design feedback tool",
    ],
    oneLiner:
      "pls-fix focuses on comment-pinning bugs. Design Mode covers comments plus full visual editing, MCP handoff, and structured exports.",
    positioning:
      "pls-fix is a specialised tool for filing visual bug reports with annotations. Design Mode is a broader design surface — comment pins are one feature among many (Layers tree, Design controls, Changes tab, MCP).",
    whenToPickDesignMode: [
      "You want to fix the bug, not just annotate it.",
      "You want to ship the structured diff to an AI agent.",
      "You want one tool for designers, developers, QA, PMs, and content.",
    ],
    whenToPickCompetitor: [
      "All you need is comment pins on screenshots.",
      "You don't need to edit the page itself.",
    ],
    table: baseFeatures({
      editing: "Comment / pin only",
      mcp: "No",
      history: "Per-comment threads",
      os: "Check current licence",
      price: "Check current pricing",
      export: "Comments / links",
      fit: "Bug reporting & feedback",
    }),
    honesty:
      "If you only need to pin comments on a deployed page, pls-fix is purpose-built. If you also want to make the fix, ship the spec to engineering, or run a full design review, Design Mode covers more ground.",
    related: [
      "design-mode-vs-drawbridge",
      "design-mode-vs-ui-inspector",
      "design-mode-vs-stagewise",
    ],
  },
  {
    slug: "design-mode-vs-drawbridge",
    competitor: "Drawbridge",
    title: "Design Mode vs Drawbridge",
    metaTitle:
      "Design Mode vs Drawbridge — visual edits to AI agents without folder lock-in",
    metaDescription:
      "Comparison: Drawbridge captures annotations and writes task files into one connected project folder. Design Mode edits the live page, hands off over MCP with no folder binding, and closes the loop by letting the agent resolve your comments.",
    keywords: [
      "Design Mode vs Drawbridge",
      "Drawbridge alternative",
      "visual annotation for AI agents",
      "drawbridge moat tasks",
      "AI coding visual feedback",
    ],
    oneLiner:
      "Drawbridge captures annotations and drops task files into your project folder; Design Mode edits the live page and hands off over MCP — no folder binding, so it works across parallel agents and worktrees.",
    positioning:
      "Both turn visual feedback into work for an AI coding agent. Drawbridge is annotate-and-queue: you comment or draw a box, it writes screenshots + `.moat/` task files into one connected project directory, and the agent reads them via a `/bridge` command. Design Mode is a live two-way loop: you edit the page directly (or pin element/region comments), and the agent pulls the diff over MCP and pushes changes back — then marks your comments resolved.",
    whenToPickDesignMode: [
      "You run multiple agents or git worktrees at once — Design Mode's MCP attaches per session with no folder binding, so nothing pins you to a single directory or risks writing tasks into the wrong tree.",
      "You want to actually edit the design (typography, colour, layout, motion, effects), not just describe the change.",
      "You want the loop to close: the agent applies changes to the live page and marks your comments resolved (mark_comment_resolved), instead of a one-way file drop.",
      "You want a persistent, searchable Changes history and CSS / Tailwind / SCSS / JSX export.",
      "You'd rather not grant a browser persistent write access to your project folder.",
    ],
    whenToPickCompetitor: [
      "You never run the MCP companion and want a zero-server flow — Drawbridge writes plain files an agent reads on its own.",
      "You want annotations checked into git as `.moat/` task files for an async, review-later workflow.",
      "Drawbridge's `/bridge` step/batch/yolo command already fits your team's habits.",
    ],
    table: baseFeatures({
      editing:
        "Annotate only — comment pins + freeform rectangles, no live editing",
      mcp: "No — writes .moat/ task files via the File System Access API",
      history: "Task files (to do / doing / done) in the connected folder",
      os: "Check current licence",
      price: "Check current pricing",
      export: "Markdown + JSON task files",
      fit: "Annotate-and-queue for AI coding agents",
    }),
    honesty:
      "Drawbridge is a clean, focused tool, and its no-server file handoff is genuinely simpler if you never run a companion: annotations land as git-friendly task files an agent reads on its own. The trade-off is the folder binding — it persists one project directory handle, which gets awkward when you're juggling parallel agents or worktrees, and it captures intent rather than letting you make the change. Design Mode deliberately skipped file handoff for exactly that reason and leans on a live MCP connection instead. If you want async, checked-in task files, Drawbridge fits; if you want to edit directly and close the loop live across many sessions, Design Mode does.",
    related: [
      "design-mode-vs-pls-fix",
      "design-mode-vs-stagewise",
      "design-mode-vs-agentation",
    ],
  },
  {
    slug: "design-mode-vs-agentation",
    competitor: "Agentation",
    title: "Design Mode vs Agentation",
    metaTitle:
      "Design Mode vs Agentation — open-source visual editor for AI agents",
    metaDescription:
      "Comparison: Agentation focuses on AI agent integration; Design Mode adds a full visual design surface, three MCP modes, and a broader persona fit.",
    keywords: [
      "Design Mode vs Agentation",
      "Agentation alternative",
      "AI agent design tool",
      "visual editor for AI agents",
    ],
    oneLiner:
      "Both tools wire visual edits into AI coding agents — Design Mode adds an open-source design surface, three MCP connection modes, and use-case-specific workflows.",
    positioning:
      "Agentation centres the agent integration; Design Mode centres the design tool that happens to integrate with agents. Different starting points; overlapping outcomes.",
    whenToPickDesignMode: [
      "You want MIT open source.",
      "You need a richer visual control set (motion, effects, contrast checker).",
      "You want three connection modes (Cloud / Local / Self-hosted) instead of a single hosted path.",
    ],
    whenToPickCompetitor: [
      "Agentation's specific agent UX matches your team better.",
    ],
    table: baseFeatures({
      editing: "Yes — agent-focused editing",
      mcp: "Yes",
      history: "Per-session",
      os: "Check current licence",
      price: "Check current pricing",
      export: "Partial",
      fit: "AI-coding-agent users",
    }),
    honesty:
      "We track Agentation's roadmap — it's a real competitor. Pick whichever fits your editing UX and licensing constraints.",
    related: [
      "design-mode-vs-stagewise",
      "design-mode-vs-dialkit",
      "design-mode-vs-cursor-design-mode",
    ],
  },
  {
    slug: "design-mode-vs-dialkit",
    competitor: "Dialkit",
    title: "Design Mode vs Dialkit",
    metaTitle: "Design Mode vs Dialkit — comparison for live website editing",
    metaDescription:
      "Comparison: Dialkit and Design Mode are both browser-based design tools. Design Mode adds MCP handoff to AI coding agents and a full Changes-tab history.",
    keywords: [
      "Design Mode vs Dialkit",
      "Dialkit alternative",
      "browser design tool",
      "live editing tool",
    ],
    oneLiner:
      "Both are browser-based visual editors. Design Mode's differentiator is the MCP handoff and the persistent, exportable change history.",
    positioning:
      "Dialkit and Design Mode share the in-browser editing premise. The fork in the road is what you do with the edits — Design Mode treats them as a structured diff to ship to engineering or an AI agent.",
    whenToPickDesignMode: [
      "You want to hand edits to Claude Code, Cursor, or any MCP agent.",
      "You need a Changes tab with search, filter, and export.",
      "You want an MIT-licensed tool.",
    ],
    whenToPickCompetitor: [
      "Dialkit's specific UX or integrations match your stack better.",
    ],
    table: baseFeatures({
      editing: "Yes",
      mcp: "No / Limited",
      history: "Limited",
      os: "Check current licence",
      price: "Check current pricing",
      export: "Limited",
      fit: "Designers / developers editing in-browser",
    }),
    honesty:
      "If you don't need AI agent handoff, Dialkit and Design Mode are both reasonable choices. If you do, Design Mode's three MCP modes are the deciding factor.",
    related: [
      "design-mode-vs-agentation",
      "design-mode-vs-stagewise",
      "design-mode-vs-ui-inspector",
    ],
  },
  {
    slug: "design-mode-vs-ui-inspector",
    competitor: "UI Inspector",
    title: "Design Mode vs UI Inspector",
    metaTitle:
      "Design Mode vs UI Inspector — design surface vs inspection tool",
    metaDescription:
      "Comparison: UI Inspector tools focus on reading CSS off live pages. Design Mode reads, edits, exports, and ships to AI agents over MCP.",
    keywords: [
      "Design Mode vs UI Inspector",
      "UI Inspector alternative",
      "Chrome inspector tool",
      "CSS inspector",
    ],
    oneLiner:
      "UI Inspector tools read CSS; Design Mode reads, edits, and ships the diff.",
    positioning:
      "Inspector tools tell you what's on the page. Design Mode lets you change it, log the change, and hand the diff to engineering or an AI agent.",
    whenToPickDesignMode: [
      "You want to make changes, not just read them.",
      "You want to ship the changes somewhere.",
      "You want an AI agent to write the production code.",
    ],
    whenToPickCompetitor: [
      "All you need is a read-only inspector and the browser already has DevTools.",
    ],
    table: baseFeatures({
      editing: "Read-only inspection",
      mcp: "No",
      history: "None",
      os: "Often free",
      price: "Often free",
      export: "Copy CSS to clipboard",
      fit: "Inspecting layouts",
    }),
    honesty:
      "If you literally just need to peek at CSS, Chrome DevTools is free and built-in. If you want to make changes that go somewhere, Design Mode is the next step up.",
    related: [
      "design-mode-vs-chrome-devtools",
      "design-mode-vs-csspeeper",
      "design-mode-vs-hover-inspector",
    ],
  },
  {
    slug: "design-mode-vs-cursor-design-mode",
    competitor: "Cursor's built-in design mode",
    title: "Design Mode vs Cursor's design mode",
    metaTitle:
      "Design Mode vs Cursor's design mode — browser surface vs editor surface",
    metaDescription:
      "Comparison: Cursor's design mode lives in the editor; Design Mode lives on the live web page. Edit the real rendered UI with real layout context, then ship the diff back to Cursor.",
    keywords: [
      "Design Mode vs Cursor design mode",
      "Cursor design mode",
      "Cursor visual editing",
      "in-browser vs in-editor design",
    ],
    oneLiner:
      "Cursor's mode lives inside the editor. Design Mode lives on the live web page — edit the real rendered UI, then ship the diff back to Cursor.",
    positioning:
      "The fork is: do you want to edit the design inside your code editor (Cursor's mode), or on the actual rendered page in your browser (Design Mode)? The answer depends on which surface gives you more context.",
    whenToPickDesignMode: [
      "You want to edit on the rendered page, not in a code-adjacent surface.",
      "You want real device pixels, real fonts, real hover/focus/animation states.",
      "You're testing across multiple pages or staging deploys.",
    ],
    whenToPickCompetitor: [
      "You stay inside Cursor all day and don't want a second tool.",
      "Your edits are local-only and never touch a deployed surface.",
    ],
    table: baseFeatures({
      editing: "In-editor design surface",
      mcp: "Native to Cursor",
      history: "Editor history",
      os: "Proprietary",
      price: "Cursor subscription",
      export: "Code changes in editor",
      fit: "Cursor power users",
    }),
    honesty:
      "Cursor's mode is great for in-editor work. Design Mode is great when you want the canvas to be the actual rendered page. Many people use both.",
    related: [
      "design-mode-vs-stagewise",
      "design-mode-vs-chrome-devtools",
      "design-mode-vs-figma-dev-mode",
    ],
  },
  {
    slug: "design-mode-vs-csspeeper",
    competitor: "CSSPeeper",
    title: "Design Mode vs CSSPeeper",
    metaTitle: "Design Mode vs CSSPeeper — design surface vs CSS inspector",
    metaDescription:
      "Comparison: CSSPeeper is a popular CSS reader; Design Mode is a full visual editor that reads, edits, and ships the diff over MCP.",
    keywords: [
      "Design Mode vs CSSPeeper",
      "CSSPeeper alternative",
      "CSS inspector",
      "Chrome CSS extension",
    ],
    oneLiner: "CSSPeeper reads. Design Mode reads, edits, and ships.",
    positioning:
      "CSSPeeper is a beautiful read-only inspector. Design Mode covers the read use case and adds editing, persistence, exports, and MCP.",
    whenToPickDesignMode: [
      "You want to change values, not just read them.",
      "You want to ship structured edits to engineering or an AI agent.",
    ],
    whenToPickCompetitor: ["You purely want to learn how a page is built."],
    table: baseFeatures({
      editing: "Read-only",
      mcp: "No",
      history: "None",
      os: "Proprietary",
      price: "Free",
      export: "Copy CSS",
      fit: "Learning, inspecting",
    }),
    honesty:
      "Different tools for different tasks. CSSPeeper is a wonderful learning aid; Design Mode is a working surface.",
    related: [
      "design-mode-vs-ui-inspector",
      "design-mode-vs-hover-inspector",
      "design-mode-vs-chrome-devtools",
    ],
  },
  {
    slug: "design-mode-vs-hover-inspector",
    competitor: "Hover Inspector",
    title: "Design Mode vs Hover Inspector",
    metaTitle:
      "Design Mode vs Hover Inspector — full design surface vs hover-only inspection",
    metaDescription:
      "Comparison: Hover Inspector tools surface element info on hover. Design Mode adds full visual editing, persistent history, exports, and AI agent handoff.",
    keywords: [
      "Design Mode vs Hover Inspector",
      "Hover Inspector alternative",
      "live element inspector",
      "CSS hover tool",
    ],
    oneLiner: "Hover-only inspectors are a feature; Design Mode is a tool.",
    positioning:
      "Hover-on-element info is one capability inside Design Mode (the inspector overlay). The broader product adds editing, exports, and MCP.",
    whenToPickDesignMode: ["You need to make changes, not just see info."],
    whenToPickCompetitor: ["You only need a hover tooltip and nothing else."],
    table: baseFeatures({
      editing: "Hover info only",
      mcp: "No",
      history: "None",
      os: "Proprietary",
      price: "Free",
      export: "None",
      fit: "Casual inspection",
    }),
    honesty:
      "If hover info is all you need, the small footprint of those extensions wins. Otherwise, Design Mode includes that capability among many.",
    related: [
      "design-mode-vs-csspeeper",
      "design-mode-vs-ui-inspector",
      "design-mode-vs-chrome-devtools",
    ],
  },
  {
    slug: "design-mode-vs-builder-io-visual-copilot",
    competitor: "Builder.io Visual Copilot",
    title: "Design Mode vs Builder.io Visual Copilot",
    metaTitle:
      "Design Mode vs Builder.io Visual Copilot — direction of design-to-code",
    metaDescription:
      "Comparison: Builder.io Visual Copilot converts Figma designs into code. Design Mode goes the other direction — tweak the live page, hand the agent precise CSS deltas.",
    keywords: [
      "Design Mode vs Builder.io",
      "Builder.io Visual Copilot alternative",
      "Figma to code",
      "design to code tool",
    ],
    oneLiner:
      "Builder.io goes Figma → code. Design Mode goes live page → code.",
    positioning:
      "Different problems. Builder.io is great for greenfield Figma-driven workflows. Design Mode is great for iterating on an already-deployed product.",
    whenToPickDesignMode: [
      "Your codebase and design system already exist.",
      "You iterate on the rendered page, not in Figma.",
      "You want an AI agent to write the change inside your existing code, not generate a new component.",
    ],
    whenToPickCompetitor: [
      "You're starting from a Figma file and want generated code.",
      "Your design source of truth lives in Figma.",
    ],
    table: baseFeatures({
      editing: "Figma → code generation",
      mcp: "Figma plugin",
      history: "Per-Figma file",
      os: "Proprietary",
      price: "Paid tiers",
      export: "Generated component code",
      fit: "Greenfield Figma workflows",
    }),
    honesty:
      "Different tools for different ends of the design-to-code spectrum. Many teams use both.",
    related: [
      "design-mode-vs-locofy",
      "design-mode-vs-figma-dev-mode",
      "design-mode-vs-stagewise",
    ],
  },
  {
    slug: "design-mode-vs-locofy",
    competitor: "Locofy",
    title: "Design Mode vs Locofy",
    metaTitle: "Design Mode vs Locofy — Figma-to-code vs live-page-to-code",
    metaDescription:
      "Comparison: Locofy converts Figma to code. Design Mode lets you edit the live page and hand the structured diff to your AI coding agent.",
    keywords: [
      "Design Mode vs Locofy",
      "Locofy alternative",
      "Figma to React",
      "design to code tool",
    ],
    oneLiner:
      "Locofy generates code from Figma. Design Mode emits diffs from the live page.",
    positioning:
      "Same comparison shape as Builder.io: Figma-first generation vs live-page-first iteration. Different starting points, different fits.",
    whenToPickDesignMode: [
      "You iterate on an existing deployed product.",
      "You want diffs your AI agent applies into existing code.",
    ],
    whenToPickCompetitor: [
      "You design in Figma and need first-pass code output.",
    ],
    table: baseFeatures({
      editing: "Figma → code generation",
      mcp: "Figma plugin",
      history: "Per-Figma file",
      os: "Proprietary",
      price: "Paid tiers",
      export: "Generated component code",
      fit: "Figma-first teams",
    }),
    honesty:
      "Both can coexist — Locofy for the first-pass component, Design Mode for the production-tuning loop.",
    related: [
      "design-mode-vs-builder-io-visual-copilot",
      "design-mode-vs-figma-dev-mode",
      "design-mode-vs-stagewise",
    ],
  },
  {
    slug: "design-mode-vs-chrome-devtools",
    competitor: "Chrome DevTools",
    title: "Design Mode vs Chrome DevTools",
    metaTitle:
      "Design Mode vs Chrome DevTools — design surface vs developer inspector",
    metaDescription:
      "Comparison: Chrome DevTools is a developer inspector. Design Mode is a design surface with persistent change history, real visual controls, and one-click ship-to-AI-agent over MCP.",
    keywords: [
      "Design Mode vs Chrome DevTools",
      "DevTools alternative",
      "Chrome design tool",
      "live CSS editing alternative",
    ],
    oneLiner:
      "DevTools is a debugger. Design Mode is a design surface that emits structured diffs and ships them to AI agents.",
    positioning:
      "Chrome DevTools is the universal default — free, built-in, every web developer knows it. Design Mode is what you reach for when DevTools' lack of persistence, lack of visual controls, and lack of agent handoff start to bite.",
    whenToPickDesignMode: [
      "You want edits that persist across reloads and sessions.",
      "You want real visual controls (sliders, colour pickers, motion, effects) instead of typed CSS strings.",
      "You want to ship the diff to an AI agent or to engineering as a structured spec.",
      "You're a designer or QA without a strong CSS background.",
    ],
    whenToPickCompetitor: [
      "You're debugging behaviour, not visuals.",
      "You need DevTools' Network / Performance / Application panels.",
    ],
    table: baseFeatures({
      editing: "Type CSS rules manually; not persistent",
      mcp: "No",
      history: "Lost on reload",
      os: "Open source (Chromium)",
      price: "Free, built-in",
      export: "Copy CSS rules",
      fit: "Developers debugging behaviour",
    }),
    honesty:
      "DevTools is irreplaceable. Design Mode lives on top of it — same browser, complementary surface. Most users keep both pinned.",
    related: [
      "design-mode-vs-cursor-design-mode",
      "design-mode-vs-figma-dev-mode",
      "design-mode-vs-visbug",
    ],
  },
  {
    slug: "design-mode-vs-figma-dev-mode",
    competitor: "Figma Dev Mode",
    title: "Design Mode vs Figma Dev Mode",
    metaTitle:
      "Design Mode vs Figma Dev Mode — file-based handoff vs live-page editing",
    metaDescription:
      "Comparison: Figma Dev Mode reads a static design file. Design Mode edits the live deployed page. Pair them: design in Figma, refine on the rendered page in Design Mode.",
    keywords: [
      "Design Mode vs Figma Dev Mode",
      "Figma Dev Mode alternative",
      "design handoff tool",
      "Figma to code workflow",
    ],
    oneLiner:
      "Figma Dev Mode reads a static design file; Design Mode edits the live deployed page.",
    positioning:
      "These tools sit on opposite ends of the design-to-code pipeline. Figma Dev Mode is where designers hand off a spec; Design Mode is where the deployed page gets refined back into the source.",
    whenToPickDesignMode: [
      "You're working on the deployed product, not the Figma file.",
      "You want edits that translate directly into PRs.",
      "You want an AI agent to write the production code from your edits.",
    ],
    whenToPickCompetitor: [
      "You're reading the design spec for a new feature.",
      "Your team's source of truth is the Figma file.",
    ],
    table: baseFeatures({
      editing: "Read-only handoff of design file",
      mcp: "No",
      history: "Figma version history",
      os: "Proprietary",
      price: "Figma seat (Dev Mode tier)",
      export: "Code snippets, measurements",
      fit: "Engineers reading a Figma spec",
    }),
    honesty:
      "Different tools, different stages. Most teams that use Figma Dev Mode will also benefit from Design Mode on the deployed side.",
    related: [
      "design-mode-vs-builder-io-visual-copilot",
      "design-mode-vs-locofy",
      "design-mode-vs-cursor-design-mode",
    ],
  },
  {
    slug: "design-mode-vs-visbug",
    competitor: "VisBug",
    title: "Design Mode vs VisBug",
    metaTitle: "Design Mode vs VisBug — live page editor with AI agent handoff",
    metaDescription:
      "Comparison: VisBug edits live pages with a small toolbar. Design Mode adds a full side panel, persistent Changes history, exports, and MCP handoff to AI coding agents.",
    keywords: [
      "Design Mode vs VisBug",
      "VisBug alternative",
      "live page editor",
      "browser design extension",
    ],
    oneLiner:
      "VisBug edits the live page but has no agent / MCP handoff and no session persistence.",
    positioning:
      "VisBug is the OG of in-browser visual editing — small toolbar, quick edits, no persistence. Design Mode is the version of that idea built for the AI-coding-agent era.",
    whenToPickDesignMode: [
      "You want edits that persist and export to a structured diff.",
      "You want to ship the diff to an AI agent.",
      "You need the full design surface (motion, effects, contrast checker, layers).",
    ],
    whenToPickCompetitor: [
      "You want a tiny zero-config toolbar and nothing more.",
    ],
    table: baseFeatures({
      editing: "Yes — toolbar-based",
      mcp: "No",
      history: "Per-session, not exportable",
      os: "Apache 2.0",
      price: "Free",
      export: "Copy CSS",
      fit: "Quick in-browser tweaks",
    }),
    honesty:
      "VisBug is fantastic and influential. Design Mode is closest in spirit to it but built for a different era and a different handoff target.",
    related: [
      "design-mode-vs-chrome-devtools",
      "design-mode-vs-stagewise",
      "design-mode-vs-cursor-design-mode",
    ],
  },
  {
    slug: "design-mode-vs-figma-make",
    competitor: "Figma Make",
    title: "Design Mode vs Figma Make",
    metaTitle:
      "Design Mode vs Figma Make — open live-page editing vs credit-metered Figma generation",
    metaDescription:
      "Figma Make generates UIs and now references local / GitHub repos — but it's locked to Figma, needs a paid seat plus metered Make credits. Design Mode is free, open-source, edits your live deployed page, and hands off to any AI agent.",
    keywords: [
      "Design Mode vs Figma Make",
      "Figma Make alternative",
      "Figma Make pricing credits",
      "Figma Make vs open source",
    ],
    oneLiner:
      "Figma Make generates UIs inside Figma on metered credits; Design Mode edits your live deployed page for free and hands the diff to any agent.",
    positioning:
      "Figma Make has grown well past basic colours and typography — it now generates whole working UIs and can even reference local or GitHub repos. The catch is where it lives: inside Figma, on a paid seat, metered by Make credits that Figma bills separately from whatever LLM you already pay for. Design Mode runs in your browser on the real deployed page, is MIT-licensed and free, and brings your own agent over MCP.",
    whenToPickDesignMode: [
      "You want to edit the actual deployed page, not regenerate it in a canvas.",
      "You don't want a second metered bill (Make credits) on top of your LLM provider.",
      "You want open source, no vendor lock-in, and a bring-your-own agent (Claude Code, Cursor, …).",
    ],
    whenToPickCompetitor: [
      "Your team already lives in Figma and wants generation native to the canvas.",
      "You're spinning up a fresh prototype from a prompt rather than refining a shipped page.",
    ],
    table: [
      ...baseFeatures({
        editing:
          "Generates UIs in Figma's canvas/files (and from local / GitHub repos) — not your live deployed page",
        mcp: "No open MCP handoff — output stays in Figma or its export",
        history: "Figma version history (inside the file)",
        os: "Proprietary — vendor-locked to Figma",
        price:
          "Paid Figma seat + metered Make credits, billed by Figma separately from your LLM provider",
        export: "Code export within Figma's ecosystem",
        fit: "Teams who live in Figma and accept credit-metered generation",
      }),
      {
        feature: "Works from your local / GitHub repo",
        designMode:
          "Edits the live rendered page; the diff plus file:line hints go to your agent to apply in the repo with the model you already pay for",
        competitor:
          "Yes — but generation is metered by Make credits and the workflow stays anchored in Figma",
      },
    ],
    honesty:
      "Figma Make is genuinely powerful inside Figma's ecosystem, and the local / GitHub-repo support is real. But it's a paid, credit-metered, Figma-locked generator. Design Mode is the opposite trade: free, open, bring-your-own-LLM, and pointed at the live page you've already shipped.",
    related: [
      "design-mode-vs-figma-dev-mode",
      "design-mode-vs-builder-io-visual-copilot",
      "design-mode-vs-locofy",
    ],
  },
  {
    slug: "design-mode-vs-editui",
    competitor: "EditUI",
    title: "Design Mode vs EditUI",
    metaTitle: "Design Mode vs EditUI — recorded edits vs written instructions",
    metaDescription:
      "Honest comparison: EditUI lets you click elements in Chrome, type an instruction per element and paste the batch into Claude Code, Cursor or Codex. Design Mode records the actual values you change, streams them over MCP, and runs on Chrome, Firefox and Safari.",
    keywords: [
      "Design Mode vs EditUI",
      "EditUI alternative",
      "EditUI Chrome extension",
      "Cursor Design Mode for any agent",
      "visual UI feedback for coding agents",
    ],
    oneLiner:
      "EditUI is the closest 'point at it' competitor: a free Chrome extension that collects per-element instructions and copies them into your agent.",
    positioning:
      "Both tools exist to stop you describing which element you mean. EditUI's answer is annotation: click, type what should change, collect, copy a prompt that carries DOM, class and computed-style context. Design Mode's answer is editing: change the value on the page, see it, and hand the agent the before/after diff over MCP. EditUI edits are still words; Design Mode edits are numbers and tokens.",
    whenToPickDesignMode: [
      "You want to try the change on the page first and hand over the exact value, not a sentence the agent has to interpret.",
      "You work in Firefox or Safari — EditUI is Chrome-only.",
      "You want the agent connected live over MCP (Cloud, Local or Self-hosted) instead of a copy-paste round trip.",
      "You want a searchable Changes tab and Markdown/JSON export of everything you touched.",
    ],
    whenToPickCompetitor: [
      "You prefer writing the instruction ('make this secondary') and letting the agent decide the implementation.",
      "You want zero setup: no MCP config, no companion process, just copy and paste.",
      "Your review is mostly non-visual (behaviour, content, structure) where a typed note is the natural unit.",
    ],
    table: baseFeatures({
      editing:
        "No — you select an element and type an instruction; the page itself is not changed",
      mcp: "No — copies a prompt to the clipboard; you paste it into Claude Code, Cursor, Codex or Windsurf",
      history:
        "Per-review list of collected edits in chrome.storage.local; cleared on uninstall",
      os: "Not stated on the site; Chrome Web Store listing only",
      price: "Free, no account (made by Usercall)",
      export: "Prompt text only (copy all)",
      fit: "Developers reviewing a running app in Chrome who paste into an agent",
    }),
    honesty:
      "EditUI's copy-everything prompt is well designed: element, text, DOM, classes, parent, neighbours, URL, viewport and computed styles. If your agent is good at turning 'tighten this spacing' into code, the copy-paste loop is fine. Design Mode's case is precision and reach: you record 16px → 12px rather than 'tighter', the agent gets it over MCP without a paste, and the same workflow runs in Firefox and Safari.",
    research: {
      checkedOn: "11 October 2026",
      methodology:
        "Documentation review of the EditUI homepage, Chrome extension page and FAQ, plus the 4 October 2026 Show HN post. The extension was not installed and no agent round-trip was tested. Capability claims are vendor-documented, not independently runtime-verified.",
      sources: [
        { label: "EditUI homepage", url: "https://www.editui.app/" },
        {
          label: "EditUI Chrome extension and FAQ",
          url: "https://www.editui.app/chrome-extension",
        },
        {
          label: "Show HN: EditUI (4 October 2026)",
          url: "https://news.ycombinator.com/item?id=49959046",
        },
      ],
    },
    related: [
      "design-mode-vs-cursor-design-mode",
      "design-mode-vs-stagewise",
      "design-mode-vs-airship",
    ],
  },
  {
    slug: "design-mode-vs-airship",
    competitor: "Airship",
    title: "Design Mode vs Airship",
    metaTitle:
      "Design Mode vs Airship — browser extension vs local design canvas",
    metaDescription:
      "Honest comparison: Airship wraps your local dev server in a Figma-like canvas and drives Claude Code, Codex or OpenCode to edit source. Design Mode is a browser extension that edits any scriptable page — localhost, staging or production — and hands the diff to any MCP client.",
    keywords: [
      "Design Mode vs Airship",
      "Airship alternative",
      "airship.design",
      "visual editor for your codebase",
      "design canvas for Claude Code",
    ],
    oneLiner:
      "Airship is a local reverse proxy that puts an infinite design canvas with real viewports in front of your dev server and lets an agent edit the source.",
    positioning:
      "Airship and Design Mode disagree about where the editor should live. Airship says: run `npx @airshiplabs/cli --target 3000`, open the canvas on localhost:3001, select an element, and the agent resolves it back to file and line and edits the source. Design Mode says: the page you already have open is the canvas, whichever origin it is on, and the extension streams what you changed to the agent over MCP.",
    whenToPickDesignMode: [
      "You need to edit a staging or production page, or any site you don't run locally — Airship is localhost-only.",
      "You want to share a session or stream edits to an agent on another machine (Cloud mode).",
      "You want the native browser, DevTools and extensions you already use rather than a proxied canvas.",
      "Your agent is not Claude Code, Codex or OpenCode — Design Mode speaks MCP to any client.",
    ],
    whenToPickCompetitor: [
      "You want side-by-side real viewports (desktop, tablet, mobile) of the same local app.",
      "You want the tool to map a selection straight to a source file and line.",
      "You want built-in per-file revert without Git and a safe mode that blocks destructive agent commands.",
      "You value a fully local, no-account, no-telemetry setup.",
    ],
    table: baseFeatures({
      editing:
        "Yes, on your own local dev server only — the canvas proxies localhost and resolves selections to source",
      mcp: "No MCP — drives Claude Code, Codex or OpenCode directly from the canvas",
      history:
        "Keeps the previous version of each edited file for instant revert; no searchable edit log documented",
      os: "CLI is published under MIT on npm (@airshiplabs/cli 0.3.0, August 2026); repository at github.com/0xnyn/airship",
      price: "Free; no account or hosted service",
      export: "Not documented — edits land directly in source",
      fit: "Developers iterating on a local React/Next/Vite app with a terminal agent",
    }),
    honesty:
      "Airship's multi-viewport canvas and file:line resolution are things Design Mode does not do, and 'edit the source, revert the file' is a tighter loop than a diff handoff when everything is on one laptop. Design Mode's case is reach: it works on pages you don't host, in three browsers, and with any MCP client, and it keeps a persistent record of what changed.",
    research: {
      checkedOn: "11 October 2026",
      methodology:
        "Documentation review of the Airship homepage, how-it-works and FAQ sections, plus the @airshiplabs/cli npm registry entry for version and licence. The CLI was not run and no agent edit was tested. Capability claims are vendor-documented, not independently runtime-verified.",
      sources: [
        {
          label: "Airship homepage and FAQ",
          url: "https://www.airship.design/",
        },
        {
          label: "@airshiplabs/cli on npm",
          url: "https://www.npmjs.com/package/@airshiplabs/cli",
        },
      ],
    },
    related: [
      "design-mode-vs-editui",
      "design-mode-vs-stagewise",
      "design-mode-vs-figma-dev-mode",
    ],
  },
  {
    slug: "design-mode-vs-layerdock",
    competitor: "LayerDock",
    title: "Design Mode vs LayerDock",
    metaTitle:
      "Design Mode vs LayerDock — visual editor vs client feedback pins",
    metaDescription:
      "Honest comparison: LayerDock pins client feedback on a live site with screenshot, device and console errors, and exposes pins to Claude Code or Cursor over MCP. Design Mode records the design change itself and hands the diff to the agent.",
    keywords: [
      "Design Mode vs LayerDock",
      "LayerDock alternative",
      "website feedback tool MCP",
      "client feedback pins Claude Code",
      "visual editor vs annotation tool",
    ],
    oneLiner:
      "LayerDock is a client-feedback tool: comments pinned to elements on a live site, with context an agent can read over MCP.",
    positioning:
      "LayerDock solves 'which button?' for clients who will never install anything: share a review link, they click and comment, each pin carries a screenshot, device, browser and page errors, and your agent can read and resolve pins through LayerDock's MCP server. Design Mode solves a different step — the person who knows what the fix is makes it on the page and the agent receives the exact change. Comments versus edits.",
    whenToPickDesignMode: [
      "The reviewer is a designer or developer who can make the change, not just describe it.",
      "You want typography, colour, spacing and layout values recorded as a diff, not a sentence.",
      "You want an open-source, free tool with no per-seat or per-dock limits.",
      "You need Firefox and Safari as first-class review browsers.",
    ],
    whenToPickCompetitor: [
      "Your reviewers are clients or stakeholders who should get a link, not an extension.",
      "You need threads, assignments, status and pushes to Slack, Jira, Linear, GitHub, Trello or ClickUp.",
      "Screenshots, recordings and captured console errors matter more than exact style values.",
    ],
    table: baseFeatures({
      editing:
        "No — comments pinned to elements; Inspect mode shows box model and computed styles but does not change them",
      mcp: "Yes — LayerDock ships an MCP server so Claude Code, Cursor or any MCP agent can list docks, read pins and resolve them",
      history: "Pins with threads, status and assignment per dock",
      os: "No — hosted SaaS",
      price:
        "Free plan (2 docks, 35 submissions/month); paid plans unlock integrations",
      export:
        "Via integrations and MCP; no documented Markdown/JSON diff export",
      fit: "Agencies, freelancers and in-house teams collecting client feedback on live sites",
    }),
    honesty:
      "If your bottleneck is getting usable feedback out of clients, LayerDock is built for that and Design Mode is not. The two can coexist: a client pins 'this feels off' in LayerDock, and the designer answers with a recorded change in Design Mode. LayerDock was in beta with around 50 users when its founder posted on DEV, so expect the product to move.",
    research: {
      checkedOn: "11 October 2026",
      methodology:
        "Documentation review of the LayerDock homepage, FAQ and pricing section, and the founder's DEV Community launch post. No account was created and the MCP server was not connected. Capability and pricing claims are vendor-documented, not independently runtime-verified.",
      sources: [
        {
          label: "LayerDock homepage, FAQ and pricing",
          url: "https://layerdock.io",
        },
        {
          label:
            "We built LayerDock to make website feedback less painful (DEV Community)",
          url: "https://dev.to/zain_khan_/we-built-layerdock-to-make-website-feedback-less-painful-4llm",
        },
      ],
    },
    related: [
      "design-mode-vs-pls-fix",
      "design-mode-vs-ui-ticket-mcp",
      "design-mode-vs-agentation",
    ],
  },
  {
    slug: "design-mode-vs-vex",
    competitor: "Vex",
    title: "Design Mode vs Vex",
    metaTitle:
      "Design Mode vs Vex — lightweight MCP handoff vs full agent orchestrator",
    metaDescription:
      "Honest comparison: Vex pairs a Chrome extension with its own Python orchestrator, Electron app and NATS bus to run Claude against structured visual edits. Design Mode is a single extension that hands the diff to whichever MCP agent you already run.",
    keywords: [
      "Design Mode vs Vex",
      "Vex alternative",
      "lukaskellerstein vex",
      "visual editing browser AI code changes",
      "agent orchestrator Chrome extension",
    ],
    oneLiner:
      "Vex captures visual edits as 12 structured action types and ships them to its own agent orchestrator, which runs Claude and streams results back.",
    positioning:
      "Vex and Design Mode agree that edits should be structured data, not prose. They part ways on how much to own. Vex is a platform: Chrome extension, FastAPI orchestrator with SQLite, Electron desktop app, NATS for streaming, Claude Agent SDK adapters, execution traces with token cost per batch, and framework detection that generates idiomatic React, Vue, Svelte, Django or Rails code. Design Mode is an extension plus an optional relay: it records the change and speaks MCP to the agent you already have.",
    whenToPickDesignMode: [
      "You already run Claude Code, Cursor, Codex, Windsurf or any MCP client and don't want a second agent runtime.",
      "You want to install from a store and be editing in a minute — Vex needs Node 20+, Python 3.11+, uv, an orchestrator and a desktop app.",
      "You need Firefox or Safari.",
      "You want an MIT licence; Vex's repository is 'All rights reserved'.",
    ],
    whenToPickCompetitor: [
      "You want the tool to run the agent for you and show full execution traces, tool calls and cost per batch.",
      "You want AI section generation and framework-aware code output built into the editor.",
      "You're comfortable self-hosting a Python service and prefer an all-in-one pipeline.",
    ],
    table: baseFeatures({
      editing:
        "Yes — resize, restyle, swap images and AI-generate sections on live pages in Chrome 116+",
      mcp: "No — REST batch submit to its own Agent Orchestrator, which runs Claude via the Agent SDK",
      history: "Batches and per-batch execution traces stored in SQLite",
      os: "Source is public on GitHub but licensed 'All rights reserved' — not open source",
      price: "Free to run; you supply the Claude credentials",
      export: "Not documented",
      fit: "Developers who want a self-hosted end-to-end visual-edit-to-code pipeline",
    }),
    honesty:
      "Vex is ambitious and its execution-trace view (steps, tool calls, tokens, cost) is something Design Mode does not offer. It is also early — version 0.1.5 at the time of checking, a handful of stars, and a non-open licence despite the public repository. Design Mode's case is that most people already have an agent; they need a clean, portable way to hand it a change, not another orchestrator.",
    research: {
      checkedOn: "11 October 2026",
      methodology:
        "Review of the Vex GitHub README (features, architecture, quick start, API reference, licence section) and repository metadata via the GitHub API. The extension, orchestrator and desktop app were not installed or run. Capability claims are as documented by the author, not independently runtime-verified.",
      sources: [
        {
          label: "Vex repository and README",
          url: "https://github.com/lukaskellerstein/vex",
        },
      ],
    },
    related: [
      "design-mode-vs-stagewise",
      "design-mode-vs-editui",
      "design-mode-vs-drawbridge",
    ],
  },
];

export function getComparison(slug: string): Comparison | undefined {
  return comparisons.find((c) => c.slug === slug);
}
