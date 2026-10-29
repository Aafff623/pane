export type BlogPost = {
  slug: string;
  title: string;
  metaTitle: string;
  metaDescription: string;
  keywords: string[];
  datePublished: string;
  excerpt: string;
  body: { heading?: string; paragraphs: string[] }[];
  related: string[];
};

export const posts: BlogPost[] = [
  {
    slug: "turn-visual-edits-into-precise-ai-prompts",
    title: "Turn visual edits into precise AI prompts",
    metaTitle: "Turn visual edits into precise AI prompts | Design Mode",
    metaDescription:
      "A practical Design Mode workflow: edit a rendered page, capture exact changes, add intent and constraints, then verify your coding agent's implementation.",
    keywords: [
      "AI UI prompts",
      "visual editing workflow",
      "Design Mode",
      "Claude Code design changes",
      "structured design feedback",
    ],
    datePublished: "2026-09-16",
    excerpt:
      "Make the change on the page. Give your coding agent the values, the context and the boundaries—not another request to make it look better.",
    body: [
      {
        paragraphs: [
          'You want the pricing cards to feel less cramped. You ask your coding agent to "give them more breathing room". It increases the gap between cards. You meant the space inside them. Neither interpretation was unreasonable; the request never made the distinction.',
          "On an existing page, you can settle that decision before asking the agent to write code. Adjust the card in the browser, look at it alongside its neighbours, and hand over the change you actually chose. Design Mode records that visual edit; your coding agent still owns the source-code implementation.",
          "The useful hand-off combines three things: an exact change, a reason for it, and a boundary around what should stay untouched. Numbers remove ambiguity about the result. Context prevents the right numbers being applied in the wrong place.",
        ],
      },
      {
        heading: "Start with the page, not a blank prompt",
        paragraphs: [
          "Open your app on a page that allows extension scripts, ideally a local development build connected to the repository your agent can access. Select the element in Design Mode and adjust its spacing, typography or other visual properties. Work on one coherent change at a time so the resulting specification stays reviewable.",
          "The surrounding page matters. A card that looks balanced alone may be too tall in a row, push its action below the fold, or wrap awkwardly beside a longer heading. Editing the rendered interface lets you judge those relationships while choosing the values.",
          "Review the Changes tab before exporting. Remove experiments you no longer want. A record of every idea you tried is not the same thing as a clear implementation request.",
        ],
      },
      {
        heading: "A worked example: give a pricing card more room",
        paragraphs: [
          "Suppose a pricing card has 16px of internal padding and a 12px gap between its content groups. You try 24px padding and a 16px gap on desktop, while keeping the compact mobile layout. These are illustrative values, not a measured result or a recommendation for every pricing page.",
          "The visual changes answer what should change. Add a short note explaining why: the plan description and price need clearer separation, but the card should keep its current typography, colours and button treatment. Also say whether this is a shared change across all plan cards or an exception for one card.",
          "A hand-written brief accompanying the exported changes could read as follows. This is an example of additional instructions, not a verbatim Design Mode export:",
          "Target: the shared pricing-card component on the pricing page. Apply the desktop spacing change to all three plan cards, not just the selected instance.",
          "Changes: increase internal padding from 16px to 24px and the gap between content groups from 12px to 16px at the existing desktop breakpoint. Keep the current mobile spacing. Preserve the title, price, colours, radius and button styling.",
          "Implementation: inspect the existing component and spacing tokens first. Reuse a token only if its resolved value matches the intended result. If the design system cannot represent the change without a new value, flag that decision rather than silently choosing the nearest size. Do not add page-wide overrides.",
          "Acceptance: check all three cards with their real content at desktop and mobile widths. Confirm that long plan names wrap cleanly, buttons remain usable by keyboard, and unrelated cards elsewhere in the app have not changed.",
        ],
      },
      {
        heading: "An illustrative hero-spacing specification",
        paragraphs: [
          "The following illustrates the shape of a mobile hero-spacing specification. It is not a current-site measurement or a verified source-change result:",
          "- section.py-24: padding-top 96px → 64px; padding-bottom 96px → 64px _(mobile · 404px)_",
          "A companion instruction could request that mobile side padding change from 24px to 16px while desktop spacing stays unchanged.",
          "Ask the agent to inspect the existing breakpoint and spacing conventions before selecting utilities. Check the intended 64px vertical and 16px side padding at a 404px viewport, and verify that desktop values remain unchanged at 1440px. Those are example acceptance checks, not reported test results.",
          "This is not a speed benchmark. The browser preview is not the repository change: after implementation, inspect the source diff and re-check the page without Design Mode overrides.",
        ],
      },
      {
        heading: "Keep the values exact and the implementation flexible",
        paragraphs: [
          "A browser selector helps identify an element. It is not necessarily the selector that belongs in your stylesheet. The rendered node may come from a shared React component, a template loop or a third-party component with its own extension points.",
          "Ask the agent to locate the source and follow its conventions. A padding change might belong in a component variant, an existing utility class or a design token. Copying a computed style into an inline override can reproduce the preview while creating the wrong maintenance burden.",
          "Scope is especially important with tokens. Updating one shared spacing token can affect many screens. If the intent is local to pricing cards, the agent should not turn it into a global design-system change without asking.",
        ],
      },
      {
        heading: "Specify the states you actually intend to change",
        paragraphs: [
          "A screenshot shows a moment. It does not establish what should happen on keyboard focus, while a button is disabled, or when a user prefers reduced motion. Include those requirements when the edit touches an interactive component; leave unrelated behaviour alone.",
          "For a hover adjustment, describe only the difference from the default state and name what must remain stable. For example: change the background on hover, but preserve the label colour and element dimensions. If you introduce a border, account for its space so the layout does not jump.",
          "Treat accessibility checks as acceptance criteria, not a promise attached to a prompt. Preserve a visible keyboard-focus indicator, check text contrast against the actual background, and test any animation with reduced motion enabled. These instructions are yours to add where relevant; do not assume an export automatically covers them or proves compliance.",
        ],
      },
      {
        heading: "Choose a hand-off that fits your setup",
        paragraphs: [
          "Copy as Prompt gives you a portable Markdown specification to paste into your coding tool alongside the intent and acceptance criteria. It does not require an MCP connection.",
          "Send to Agent uses Design Mode's MCP connection so a configured client can read the session. The connection does not grant repository access: your coding agent separately needs permission to inspect and edit the project. Use the current MCP setup guide for your client rather than assuming every client accepts the same configuration.",
          "Before either hand-off, review what you are sharing. Page content, comments and screenshots can contain confidential information. Use representative test data and follow your team's rules for the agent provider and connection mode.",
        ],
      },
      {
        heading: "Verify the source change, not just the browser preview",
        paragraphs: [
          "Design Mode changes the rendered page as a preview. That preview is not evidence that the repository now produces the same result. After the agent implements the request, inspect the source diff, run the relevant project checks, and reopen the app without the temporary Design Mode overrides affecting the comparison.",
          "Compare the result at the same viewport size, with the same content and fonts loaded. Check the edited values in browser developer tools where useful, then inspect the overall layout. Matching a padding value does not guarantee matching line wrapping or card height.",
          "Test a neighbouring viewport as well as the one you designed at. Exercise hover, keyboard focus and disabled states when applicable. If the agent changed a shared component, inspect another place that uses it. Report a mismatch with the element, state and expected value instead of restarting with a vague visual request.",
          "There is no guaranteed one-shot result here. Precise changes make the request easier to implement and the outcome easier to check; they do not remove the need to understand the codebase or review the work.",
        ],
      },
      {
        heading: "Try it on one change",
        paragraphs: [
          "Pick a spacing or typography adjustment on a page you own. Make it visually, clean up the Changes tab, and copy the specification. Add one sentence about intent, one about scope and a short acceptance check. Ask your coding agent to implement it, then review the result without the preview overrides.",
          "That is the distinction worth keeping: Design Mode captures the visual decision, while the agent translates it into maintainable source code. A useful prompt connects those two jobs without pretending they are the same job.",
        ],
      },
    ],
    related: [
      "vibe-coding-visual-editing-workflow",
      "redesigning-a-tailwind-landing-page-with-claude-code",
    ],
  },
  {
    slug: "why-we-built-an-mcp-server-for-design-edits",
    title: "Why we built an MCP server for design edits",
    metaTitle:
      "Why we built an MCP server for design edits — Design Mode's origin story",
    metaDescription:
      "The story behind Design Mode's MCP integration: how visual design edits got first-class agent handoff, why MCP was the right protocol, and the three-mode (Cloud / Local / Self-hosted) architecture.",
    keywords: [
      "MCP server",
      "Model Context Protocol",
      "design MCP",
      "Design Mode origin",
      "AI agent design tool",
    ],
    datePublished: "2026-03-12",
    excerpt:
      "Why a browser visual editor offers both a portable copied prompt and an optional MCP connection to coding clients.",
    body: [
      {
        paragraphs: [
          "Copy as prompt copies a Markdown specification that you can paste into your coding tool. MCP offers a different handoff: a configured client can read the session directly. This article explains the design rationale, not a version-by-version release chronology.",
          "When Anthropic released Model Context Protocol in late 2024, it was obvious in retrospect what the right surface was. MCP is a standard for letting agents call tools. A design edit is a tool call. Match made.",
        ],
      },
      {
        heading: "Why MCP and not a custom protocol",
        paragraphs: [
          "We could have built a bespoke WebSocket protocol and an SDK per client. We chose MCP because it provides a shared tool boundary and supports both local stdio and remote HTTP transports, while still requiring client-specific configuration and verification.",
          "The current tool catalogue is documented on the MCP page. Those tools do not themselves grant filesystem or general network access; a coding client may have separate capabilities and repository permissions outside Design Mode.",
        ],
      },
      {
        heading: "Three connection modes",
        paragraphs: [
          "Local mode uses a stdio MCP server on your machine. Design Mode MCP traffic stays on localhost, latency is low, and the agent must run on the same machine. The trade-off is a steeper setup for people who are not comfortable with terminal commands.",
          "Cloud mode exposes a Streamable HTTP endpoint at mcp.designmode.app and uses an authenticated event stream for the extension. Consult the canonical privacy disclosure for relay handling and retention limits. A client-specific configuration and bearer token are required; no local companion process is needed.",
          "Self-hosted mode is the same Cloud relay code (packages/mcp-cloud) deployed on infrastructure you operate. For teams who want Cloud ergonomics but their own infra.",
        ],
      },
      {
        heading: "What changed in the loop",
        paragraphs: [
          "Before MCP, sending edits felt like a tax. With MCP, the hand-off can become the natural end of a design iteration: make the visual change, mark the session ready, then ask the connected client to read it and implement the source diff in a repository it can access.",
          "MCP gives Design Mode a standard boundary for tools and prompts, but each client still has its own configuration, transport and authentication rules. The integration must be verified client by client.",
        ],
      },
    ],
    related: [
      "vibe-coding-visual-editing-workflow",
      "redesigning-a-tailwind-landing-page-with-claude-code",
    ],
  },
  {
    slug: "vibe-coding-visual-editing-workflow",
    title:
      "Vibe coding: how visual editing fits into the AI-coding-agent workflow",
    metaTitle:
      "Vibe coding workflow — visual editing with Claude Code or Cursor",
    metaDescription:
      "A practical guide to using browser visual editing with verified clients such as Claude Code or Cursor while keeping the repository hand-off explicit.",
    keywords: [
      "vibe coding",
      "vibe coding workflow",
      "AI coding agent design",
      "Claude Code workflow",
      "Cursor workflow",
      "visual editing AI",
    ],
    datePublished: "2026-04-05",
    excerpt:
      "Vibe coding is what happens when you describe a UI change to an AI agent and ship the result. Visual editing is what makes the loop tight.",
    body: [
      {
        paragraphs: [
          "Vibe coding is a meme that turned into a real practice. The idea: you describe a UI change to an AI coding agent in plain English, the agent writes the code, you ship. Coined by Andrej Karpathy in early 2025; widely adopted by indie hackers, design engineers, and anyone tired of writing CSS by hand.",
          "The catch is that vibe coding only feels good when the agent gets your intent right on the first try. Otherwise you spend the same time you would have writing CSS — except now you're prompt-engineering instead of coding. That's a worse trade.",
        ],
      },
      {
        heading: "The intent gap",
        paragraphs: [
          "Exact values can make a visual request easier to interpret, but the agent still needs codebase context and review. Three common ways to communicate intent:",
          "1. Screenshots. Useful visual context, but a single frame does not specify an exact CSS property or every hover and motion state. Pair screenshots with values and state descriptions.",
          "2. Figma mockups. High-fidelity, but maintaining a Figma file that matches production is its own job. Indie hackers don't have time.",
          '3. Prose descriptions. "Make the hero pop more." "Tighten the spacing." Imprecise, slow, error-prone.',
          "Visual editing on the live page is a fourth option: tweak the real rendered surface, capture a structured diff, hand it to the agent.",
        ],
      },
      {
        heading: "What the loop looks like",
        paragraphs: [
          "Open the page you're iterating on in your browser. Open Design Mode's side panel. Make the change visually — drag a handle, pick a colour, adjust spacing. Every edit lands in the Changes tab as a structured row: selector, property, before, after.",
          "Use Send to Agent with a verified client such as Claude Code or Cursor. It can read the live session through MCP, find the source file through its separate repository access, and propose the production change for you to review.",
          'The loop is fast because the spec is exact. The agent isn\'t guessing what "more breathing room" means; it has a literal `padding-block: 24px` to write.',
        ],
      },
      {
        heading: "Where it doesn't work",
        paragraphs: [
          "Greenfield UI from scratch — Design Mode needs a page to edit. Start with a prototype or another creation tool, then review the rendered result.",
          'Pure logic changes — Design Mode is for visual surface. "Fix this race condition" is a job for the agent alone.',
          "Where a design system mandates tokens, provide its reference and ask the agent to inspect the source definitions. Verify that the selected token resolves to the intended value; instructions do not guarantee the correct utility choice.",
        ],
      },
    ],
    related: [
      "why-we-built-an-mcp-server-for-design-edits",
      "redesigning-a-tailwind-landing-page-with-claude-code",
    ],
  },
  {
    slug: "design-mode-1-5-0-changelog-deep-dive",
    title: "Design Mode 1.5.0 — changelog deep-dive",
    metaTitle:
      "Design Mode 1.5.0 changelog deep-dive — margin overlays, file size, WCAG contrast",
    metaDescription:
      "What shipped in Design Mode 1.5.0: margin/padding overlay bands, file size next to resolution in the Media section, the WCAG contrast checker in the colour picker, and the static OG image.",
    keywords: [
      "Design Mode 1.5.0",
      "Design Mode changelog",
      "Design Mode release notes",
      "WCAG contrast checker",
      "margin padding overlay",
    ],
    datePublished: "2026-05-15",
    excerpt:
      "Margin/padding overlay bands, a WCAG contrast checker in the colour picker, file-size hints in Media, and the static OG image migration.",
    body: [
      {
        paragraphs: [
          "Design Mode 1.5.0 is a small-features-only release. No protocol changes, no breaking changes — just quality-of-life upgrades that compound the daily loop.",
        ],
      },
      {
        heading: "Margin & padding overlay bands",
        paragraphs: [
          "The selection overlay now draws coloured bands for margin and padding. Previously you had to mentally subtract content-box from border-box to figure out spacing. Now it's directly visible on the canvas, colour-keyed to the design tokens in Settings.",
          "The bands respect the overlay colours you set in Settings, so they match the rest of your inspector palette. Default colours follow the Figma convention (greenish for padding, orange-ish for margin), but every overlay colour is now configurable.",
        ],
      },
      {
        heading: "File size next to resolution in Media",
        paragraphs: [
          'When you select an image, the Media section now shows file size alongside resolution. Useful for catching unoptimized assets during design review — "that\'s a 4 MB JPG" is a faster diagnosis than waiting for Lighthouse to flag it.',
        ],
      },
      {
        heading: "WCAG contrast checker in the colour picker",
        paragraphs: [
          "The colour picker now displays the WCAG 2.2 contrast ratio against the inferred background colour, with AA / AAA tags. The check happens live as you drag — pick a colour that fails, see the failure immediately.",
          "Not a replacement for a full a11y audit, but a fast way to avoid shipping low-contrast text in the first place.",
        ],
      },
      {
        heading: "Static OG image",
        paragraphs: [
          "The marketing site now uses a single static social-preview image across every page. The dynamic per-page OG generation that some of the templates ship with was costing edge function invocations for no measurable benefit; one well-designed static image is enough.",
        ],
      },
      {
        heading: "Historical scope",
        paragraphs: [
          "These notes describe 1.5.0, not the current roadmap. Earlier plans are not evidence that a capability shipped; consult the changelog and current feature documentation for release status.",
        ],
      },
    ],
    related: [
      "why-we-built-an-mcp-server-for-design-edits",
      "vibe-coding-visual-editing-workflow",
    ],
  },
  {
    slug: "redesigning-a-tailwind-landing-page-with-claude-code",
    title:
      "Illustrative walkthrough: redesigning a Tailwind landing page with Claude Code",
    metaTitle:
      "Redesigning a Tailwind landing page with Claude Code and Design Mode",
    metaDescription:
      "An illustrative Tailwind workflow: preview browser changes, ask Claude Code to map exact values into source, then review tokens, shared scope and responsive behaviour.",
    keywords: [
      "Tailwind redesign",
      "Tailwind + Claude Code",
      "redesign with AI",
      "vibe coding Tailwind",
      "Tailwind visual editor",
    ],
    datePublished: "2026-05-20",
    excerpt:
      "An illustrative workflow for previewing Tailwind page changes, asking Claude Code to map them into source, and reviewing the result. Not a measured case study.",
    body: [
      {
        paragraphs: [
          "This illustrative walkthrough shows how to specify a change on a rendered Tailwind page. It is not a report of a completed project or a timing benchmark. Design Mode records the desired browser result; Claude Code needs separate repository access to implement it.",
        ],
      },
      {
        heading: "Setup",
        paragraphs: [
          "Open the landing page on localhost or staging. Open the Design Mode side panel. Follow the current MCP setup guide to configure Claude Code for the mode you choose. Confirm the MCP status chip is connected.",
        ],
      },
      {
        heading: "Hero",
        paragraphs: [
          "Start with the headline. Adjust the type size visually until it feels right — drag the slider, see the rendered size, settle. Pick the colour with the colour picker; the contrast indicator is a spot-check against an inferred background, not proof of AA contrast across a gradient. Verify text against the actual colours behind it, including responsive and interactive states.",
          "Adjust hero padding using the visual spacing controls. Drop the hero image's `border-radius` from `1rem` to `0.75rem`. Every edit lands in the Changes tab.",
        ],
      },
      {
        heading: "Send to Claude Code",
        paragraphs: [
          "After Send to Agent marks the session ready, ask Claude Code to read the changes through MCP, find the JSX file and propose the utility-class update. Type sizes can map to a Tailwind step or arbitrary value; colours and spacing should be checked against the project's actual tokens and conventions.",
          "Review the source diff and run project checks. Reopen the page without Design Mode overrides at desktop and mobile widths. Commit only after the implementation is accepted.",
        ],
      },
      {
        heading: "Sections",
        paragraphs: [
          "Repeat for another section only after checking the first implementation. Keep each request scoped and review effects on shared components. No elapsed-time result is claimed for this illustrative workflow.",
        ],
      },
      {
        heading: "The pattern",
        paragraphs: [
          "The browser specification is an input, not a guarantee of clean code. Ask the agent to inspect the project’s actual Tailwind configuration and token definitions, preserve matching utilities, flag unmatched values, and avoid unrelated changes. Review the diff rather than assuming it contains only utility-class edits.",
        ],
      },
    ],
    related: [
      "turn-visual-edits-into-precise-ai-prompts",
      "vibe-coding-visual-editing-workflow",
    ],
  },
  {
    slug: "design-mode-1-9-0-release",
    title:
      "Design Mode 1.9.0 — design tokens, trigger-first motion, and a dedicated MCP page",
    metaTitle:
      "Design Mode 1.9.0 changelog deep-dive — design-system tokens, trigger-first motion, dedicated MCP page",
    metaDescription:
      "What shipped in Design Mode 1.9.0: a design-system-aware token engine with swap/edit/detach badges, trigger-first Motion interaction cards, a dedicated in-extension MCP page, a step-based Send to Agent modal, and a Select matching layers checkbox.",
    keywords: [
      "Design Mode 1.9.0",
      "Design Mode changelog",
      "Design Mode release notes",
      "design system tokens",
      "trigger-first motion",
      "MCP page",
    ],
    datePublished: "2026-07-19",
    excerpt:
      "A token engine that understands design systems, Motion rebuilt around triggers instead of raw CSS properties, and MCP configuration moved out of Settings into its own page.",
    body: [
      {
        paragraphs: [
          "Design Mode 1.9.0 is the biggest release since MCP shipped. Three systems got rebuilt from the ground up — tokens, Motion, and the agent handoff — plus a handful of smaller fixes that had been on the list for a while.",
        ],
      },
      {
        heading: "Design-system aware tokens",
        paragraphs: [
          "Token discovery inspects accessible stylesheet rules and inline declarations, including scopes beyond :root. Matching theme, component, media and support rules can contribute tokens; inaccessible cross-origin stylesheets and browser CSSOM restrictions can limit what is discovered.",
          "Recognised systems — IBM Carbon, Material, MUI, Bootstrap, Polaris, Radix, shadcn/ui, and Tailwind v4 — get labelled by name, and any Design-tab field authored from a variable now shows a ◆ badge. Click it for Swap token… (colour, spacing, radius, typography, and shadow all get a matched picker now, not just colour), Edit token globally, or Detach from token.",
          "Because a token is one value per theme, edits write into a managed override stylesheet scoped to wherever the element actually resolves the token, instead of a single inline override that a theme scope's own rule would just beat. The agent side gets the same context: get_changes now carries tokenChanges with the scope and system, and the /design-mode workflow tells the agent to edit the token's definition, not restyle the component.",
        ],
      },
      {
        heading: "Motion, trigger-first",
        paragraphs: [
          "Motion used to be a list of raw CSS editors — Transition, Animation, Transform. Useful, but it made you translate 'fade in on hover' into property names yourself. The Motion section now leads with interaction cards keyed by trigger: Hover, Press, and Focus animate to a target state; Appear animates from a start state on mount via @starting-style; Loop plays an infinite keyframe; Scroll drives an animation-timeline: view().",
          "Each card has change presets (Fade, Lift, Scale, Background), a shared easing Curve, a plain-English summary, and a Preview button that actually plays the interaction. The raw per-property editors didn't go away — they moved under Motion → Advanced for anyone who wants a specific CSS longhand.",
        ],
      },
      {
        heading: "A dedicated MCP page",
        paragraphs: [
          "MCP configuration — connection mode, port, auto-connect, token/tenant, Copy config / Copy token / Revoke — moved out of Settings into its own full-panel page, opened from the header MCP chip. Settings was getting crowded, and MCP setup is a distinct enough task (and one people revisit more than the rest of Settings) to earn its own screen.",
        ],
      },
      {
        heading: "Step-based Send to Agent, and a real handoff signal",
        paragraphs: [
          "Send to Agent is now a guided, step-based modal instead of a single click into the void. And on the protocol side, get_changes and get_session_summary expose a real handoff field once you send — an explicit 'these are ready' signal the agent can key off, instead of inferring intent from whatever's sitting in the change buffer.",
        ],
      },
      {
        heading: "Select matching layers",
        paragraphs: [
          "The similarity wand and threshold slider are gone, replaced by a Select matching layers checkbox in the indicator chip's Selected row. Tick it and every layer matching the same tag/class as your current selection joins it — same outcome, fewer moving parts.",
        ],
      },
      {
        heading: "Everything else",
        paragraphs: [
          "Layers tab rows no longer scroll horizontally on long names. Appearance's opacity and corner-radius fields are icon-led now, with blend mode and isolation moved into Advanced. Layout guides gained visibility gating so the section-level eye only shows up once you actually have more than one guide. And the whole panel picked up a Figma-aligned light/dark palette to cut down on visual noise.",
        ],
      },
    ],
    related: [
      "why-we-built-an-mcp-server-for-design-edits",
      "design-mode-1-5-0-changelog-deep-dive",
    ],
  },
  {
    slug: "design-mode-2-0-0-release",
    title:
      "Design Mode 2.0.0 — now on Firefox, with alignment guides and a calmer color picker",
    metaTitle:
      "Design Mode 2.0.0 — Firefox support, alignment guides, compact color picker",
    metaDescription:
      "Design Mode 2.0.0 brings the extension to Firefox (Gecko MV3) from a single shared build, adds Slides/Keynote-style alignment guides while dragging, and slims the color picker down to a compact solid picker with the full HSV surface behind a disclosure.",
    keywords: [
      "Design Mode 2.0.0",
      "Design Mode Firefox",
      "Firefox extension for designers",
      "browser extension for designers",
      "alignment guides",
      "Design Mode changelog",
    ],
    datePublished: "2026-07-30",
    excerpt:
      "The big one: Design Mode now runs on Firefox, from the same single build that already serves Chrome. Plus alignment guides while you drag, and a color picker that stops shouting at you.",
    body: [
      {
        paragraphs: [
          "Design Mode 2.0.0 is a milestone release — the extension is no longer Chrome-only. It now installs on Firefox too, and it does so without a second codebase or a separate build. Alongside the Firefox launch, dragging an element now snaps to alignment guides the way Slides and Keynote do, and the color picker opens in a much lighter compact form.",
        ],
      },
      {
        heading: "Firefox, from a single build",
        paragraphs: [
          "Design Mode now installs on Firefox 121+ from Firefox Add-ons (AMO), right alongside the Chrome Web Store build. The important part: it ships from one dist/ and one merged manifest.json. There is no separate Firefox build — the same bundle serves both browsers and detects the platform at runtime, with browser-dependent controls gated at runtime. This describes the 2.0 release architecture, not a guarantee of permanent feature parity.",
          "On Firefox the panel renders as the browser's native sidebar; on Chrome it's the native side panel. Alt+D opens it on both. A few things are genuinely Chrome-only because Firefox has no equivalent API — the pop-out floating window, the always-on-top Picture-in-Picture window, and the screen eyedropper — and those controls are simply hidden on Firefox rather than shown broken.",
        ],
      },
      {
        heading: "Alignment guides while you drag",
        paragraphs: [
          "Drag the body of a selected element and its edges and centres now snap to its siblings and to the parent box, with a solid magenta guide drawn through each match — the same feel as dragging a shape around in Slides or Keynote. Snapping is per-axis and composes with the existing Shift axis-lock, so you can constrain to one direction and still snap along it. Hold Alt to drag freely with snapping off, and the guides clear the moment you drop. Multi-select drags snap as a group.",
        ],
      },
      {
        heading: "A calmer color picker",
        paragraphs: [
          "Clicking a color swatch used to throw the full HSV square, hue slider, and numeric channels at you every time. Now the picker opens compact — a live swatch, a Hex field, the eyedropper (Chrome only), and the HEX/RGB/HSL format cycle, with the site's design-token colors listed below. The full HSV surface is still one click away behind a Custom colour disclosure; it's just collapsed by default so the common case stays quiet.",
        ],
      },
      {
        heading: "Everything else",
        paragraphs: [
          "Undo/redo got more reliable — it no longer injects literal HTML tags into text, and style/resize/move steps replay through the change-tracker so the page and the Changes tab never drift apart. Escape now drops back to hover mode instead of turning inspection off. With a multi-selection active, a plain click collapses the set to the one element you clicked (Shift-click still adds). Resizing past a min/max cap now widens the cap so the element actually moves. Aligning a single flex child works. Drop shadows on text layers hug the glyphs instead of boxing the text. And the texture effect is visible again, with an Opacity control.",
        ],
      },
    ],
    related: [
      "design-mode-1-9-0-release",
      "why-we-built-an-mcp-server-for-design-edits",
    ],
  },
];

export function getPost(slug: string): BlogPost | undefined {
  return posts.find((p) => p.slug === slug);
}
