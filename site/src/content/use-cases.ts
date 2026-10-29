export type UseCase = {
  slug: string;
  persona: string;
  title: string;
  metaTitle: string;
  metaDescription: string;
  keywords: string[];
  intro: string;
  problem: string;
  workflow: { name: string; text: string }[];
  outcome: string;
  related: string[];
};

export const useCases: UseCase[] = [
  {
    slug: "vibe-coding-with-claude-code",
    persona: "Vibe coders & AI-coding-agent users",
    title: "Vibe coding with Claude Code",
    metaTitle: "Vibe coding with Claude Code — visual editing + AI agent loop",
    metaDescription:
      "Use Design Mode, a browser extension for Chrome, Firefox and Safari, with Claude Code: edit a scriptable page visually, hand the structured diff over MCP, then review Claude Code's repository implementation.",
    keywords: [
      "vibe coding",
      "Claude Code visual editor",
      "Claude Code MCP",
      "AI coding agent UI",
      "design with AI agent",
    ],
    intro:
      "Design Mode is a browser extension for Chrome, Firefox and Safari that edits the rendered page. Claude Code is a coding agent that implements changes in a repository it can access. The loop is: decide the look in the browser, ask Claude Code to implement a scoped change, then review the source diff. Design Mode is not Claude's editor, and it does not write your files.",
    problem:
      'Claude Code is strong at writing CSS once it knows exactly what you want. The bottleneck is conveying design intent. Screenshots are ambiguous. Mock files drift from the production page. "Make the hero pop more" is too vague.',
    workflow: [
      {
        name: "Open a scriptable page in your browser",
        text: "Browse to your dev server, staging URL, or a production page the extension can script. Browser-protected surfaces are out of scope. Open the Design Mode side panel.",
      },
      {
        name: "Make the change visually",
        text: "Click the element. Drag handles to resize, pick colours, adjust spacing, change typography. Every edit lands in the Changes tab as a structured diff.",
      },
      {
        name: "Send to Claude Code over MCP",
        text: "With Claude Code connected through Cloud, Local, or Self-hosted MCP, click Send to Agent to mark the session ready. Ask Claude Code to read the selectors and properties, implement only that scoped change in the repository it already has access to, and show you the diff. Setup lives on /mcp.",
      },
      {
        name: "Review the repository change",
        text: "Inspect the source diff, then reload without Design Mode overrides. The browser preview is not evidence that production source already matches. Tweak in Design Mode and resend if needed.",
      },
    ],
    outcome:
      "Claude Code receives selectors and values instead of a screenshot guess. You still review the implementation in the repository; a closer first pass is not a timed guarantee.",
    related: [
      "visual-editing-with-cursor",
      "tailwind-component-tuning",
      "redesign-any-website",
    ],
  },
  {
    slug: "visual-editing-with-cursor",
    persona: "Cursor users",
    title: "Visual editing with Cursor",
    metaTitle: "Visual editing with Cursor — MCP-powered UI design loop",
    metaDescription:
      "Use Design Mode, a browser extension, with Cursor: edit a scriptable webpage, send the structured diff over MCP, then review Cursor's repository implementation. Distinct from Cursor's built-in design mode.",
    keywords: [
      "Cursor visual editor",
      "Cursor MCP",
      "Cursor design tool",
      "Cursor UI editing",
      "Cursor + Design Mode",
    ],
    intro:
      "Cursor's built-in design mode lives inside the editor. Design Mode is a browser extension that edits the actual rendered page. Use them together when you want live layout, fonts, and states in the browser, then ask Cursor to implement only the scoped change in the repo you already opened. Cursor does not commit automatically.",
    problem:
      "Cursor's chat is text-first. Describing a hover state, a gradient, or a kerning tweak in prose is slow and imprecise. Pasting screenshots works for big changes but falls apart at the pixel level. Cursor's editor-native design surface is also not the live rendered page.",
    workflow: [
      {
        name: "Point Cursor at Design Mode",
        text: "Add the Cloud or Local snippet from /mcp into ~/.cursor/mcp.json. Restart Cursor.",
      },
      {
        name: "Edit the page in the browser",
        text: "Open the Design Mode side panel on a scriptable page such as your dev server. Make the visual change with sliders and pickers. Browser-protected pages are out of scope.",
      },
      {
        name: "Ask Cursor to implement, then review",
        text: "From a Cursor chat, ask the agent to apply only the latest Design Mode changes in the current repository. It can call get_changes over MCP and write CSS where it has access. Review the source diff before you commit.",
      },
    ],
    outcome:
      "Design Mode specifies the visual change on the rendered page; Cursor implements it only where it has repository access. You review and commit. The two surfaces can coexist: editor-native for in-Cursor work, the extension for the live page.",
    related: [
      "vibe-coding-with-claude-code",
      "tailwind-component-tuning",
      "redesign-any-website",
    ],
  },
  {
    slug: "redesign-any-website",
    persona: "Designers",
    title: "Redesign any website",
    metaTitle:
      "Redesign any website — browser-as-design-surface with Design Mode",
    metaDescription:
      "Open a scriptable URL in your browser, click anything, and redesign it visually — no source code, no Figma mock. Every change is a structured preview you can hand to your team or AI agent.",
    keywords: [
      "redesign a website",
      "in-browser website editor",
      "live website design tool",
      "edit any website CSS",
      "browser extension for designers",
    ],
    intro:
      "Most design tools want you to start in a file. Design Mode lets you start on a rendered page the extension can script — your site, a staging URL, or another scriptable page — and redesign it visually in your browser. It does not bypass browser protections, and the preview is not a source change.",
    problem:
      "Recreating a real page in Figma so you can iterate on it is enormous setup cost. By the time the mock is ready, the live page has changed. Iteration in a mock that doesn't match production means decisions that don't translate.",
    workflow: [
      {
        name: "Open the page",
        text: "Browse to a localhost, staging, or production URL the extension can script. Open the Design Mode side panel.",
      },
      {
        name: "Click anything; edit anything",
        text: "Typography, colour, layout, spacing, motion, effects, copy, DOM. Every input is a real control — not a CSS textarea.",
      },
      {
        name: "Compare before / after",
        text: "Use the screenshot button to capture before, make your edits, capture after.",
      },
      {
        name: "Export the spec",
        text: "Hand a structured diff to engineering, or to a coding agent that has separate repository access. Someone still reviews the source change.",
      },
    ],
    outcome:
      "You designed on the rendered page. Shipping still requires a source change; the browser preview is not production.",
    related: [
      "design-review-in-production",
      "tailwind-component-tuning",
      "landing-page-iteration-for-indie-hackers",
    ],
  },
  {
    slug: "design-review-in-production",
    persona: "Designers & design ops",
    title: "Design review in production",
    metaTitle:
      "Design review in production — annotate the live deploy, ship a structured diff",
    metaDescription:
      "Walk your team through a deployed app, annotate every drift from the design system, and export a structured diff for engineering. Design Mode replaces the screenshot-plus-Linear-ticket loop.",
    keywords: [
      "design review tool",
      "design QA",
      "live design review",
      "production design audit",
      "design handoff",
    ],
    intro:
      "Design reviews on staging always end the same way: a Loom recording, a Notion doc full of screenshots, and engineering scrambling to interpret ambiguous arrows. Design Mode collapses that into one structured artefact.",
    problem:
      'Verbal feedback is lossy. "This needs more breathing room" can mean any of five different changes depending on who reads it. Engineers waste cycles asking which spacing token.',
    workflow: [
      {
        name: "Walk the deployed app",
        text: "Open the Design Mode side panel on each page in scope. Comment pins on issues; edits for fixes you can already make.",
      },
      {
        name: "Capture before / after",
        text: "Screenshots and the Changes tab automatically build a per-page audit trail.",
      },
      {
        name: "Export the spec",
        text: "Markdown export with selector → property → value lines. Paste straight into Linear, GitHub, or Jira.",
      },
    ],
    outcome:
      'Engineering knows exactly what to change. No "what did you mean by this?" follow-up.',
    related: [
      "ui-testing-export-to-developers",
      "bug-report-with-visual-diff",
      "design-system-audit",
    ],
  },
  {
    slug: "tailwind-component-tuning",
    persona: "Frontend developers",
    title: "Tailwind component tuning",
    metaTitle:
      "Tailwind component tuning — edit shadcn / Tailwind UI visually with AI agent handoff",
    metaDescription:
      "Tune Tailwind / shadcn components visually in the browser, then have Claude Code or Cursor write the utility classes. The fastest way to refine a Tailwind UI without leaving the page.",
    keywords: [
      "Tailwind visual editor",
      "shadcn visual editor",
      "tune Tailwind components",
      "edit Tailwind in browser",
      "Tailwind + AI agent",
    ],
    intro:
      "Tailwind makes building UI fast and tuning it slow — once you have utility classes, you're guessing at numeric increments. Design Mode gives you a visual handle on every Tailwind class and emits the diff your AI agent can translate back to utilities.",
    problem:
      "Adjusting `space-y-4` to something between 16 and 20 pixels means editing classes, refreshing, and eyeballing. Multiply that across a real component and the loop kills momentum.",
    workflow: [
      {
        name: "Open the component in the browser",
        text: "Open your storybook or your live page. Open the side panel.",
      },
      {
        name: "Tune visually",
        text: "Drag handles, change colours, adjust spacing in pixel increments.",
      },
      {
        name: "Hand the diff to your agent",
        text: "Claude Code or Cursor reads the structured diff and translates the pixel/colour values into the closest Tailwind utility classes (or your tokens) in your source.",
      },
    ],
    outcome:
      "Visual control of a utility-class UI. Your design system constraints are respected; you just spec from the rendered surface.",
    related: [
      "vibe-coding-with-claude-code",
      "visual-editing-with-cursor",
      "design-system-audit",
    ],
  },
  {
    slug: "figma-to-code-without-figma",
    persona: "Solo makers & small teams",
    title: "Figma-to-code without Figma",
    metaTitle:
      "Figma-to-code without Figma — design on the live page, ship to your AI agent",
    metaDescription:
      "Skip the Figma round-trip. Edit the real page visually, hand the structured diff to your AI agent, and let it write the code. A faster design-to-code path for small teams and indie hackers.",
    keywords: [
      "Figma alternative",
      "design without Figma",
      "Figma to code",
      "design-to-code",
      "live design tool",
    ],
    intro:
      "Figma is a great file format. It's not always the right tool for a one-person team iterating on a real, deployed product. Design Mode lets you skip the mock layer entirely.",
    problem:
      "Maintaining a Figma file that matches production is its own job. Small teams can't afford the round-trip; indie hackers don't have time for the discipline.",
    workflow: [
      {
        name: "Edit the live page",
        text: "Open Design Mode on your dev server or staging URL. Make the change visually.",
      },
      {
        name: "Ship to your AI agent",
        text: "Send to Agent marks the session ready for a connected client such as Claude Code or Cursor. The client can retrieve the changes and implement them only when it has separate repository access.",
      },
      {
        name: "Optional: use Figma for greenfield",
        text: "Keep Figma for blank-canvas exploration. Use Design Mode for everything that already exists.",
      },
    ],
    outcome:
      "Less context-switching, less file drift, more shipped iterations per week.",
    related: [
      "vibe-coding-with-claude-code",
      "landing-page-iteration-for-indie-hackers",
      "redesign-any-website",
    ],
  },
  {
    slug: "ui-testing-export-to-developers",
    persona: "QA / UI testers",
    title: "UI testing — export visual bugs to developers with full context",
    metaTitle:
      "UI testing — export visual bug reports to developers with full context",
    metaDescription:
      "QA testers and designers annotate broken layout, contrast, and copy on a staging URL with Design Mode, then export a structured diff (selector → property → value) so developers see exactly what to change.",
    keywords: [
      "UI testing",
      "visual bug report",
      "QA design tool",
      "design handoff with full context",
      "staging URL annotation",
      "structured bug report",
      "visual diff for developers",
    ],
    intro:
      "Visual bugs are the worst category of bug to report and the worst to receive. A screenshot plus a sentence isn't enough context; a Loom video is too much. Design Mode gives you a third option — annotate visually and export a precise, structured diff that ships with every detail a developer needs.",
    problem:
      "Most UI bug reports are screenshot + prose: \"the button is misaligned on mobile.\" Engineering opens the page, can't reproduce, asks for repro steps, gets a Loom, still can't tell which spacing value is wrong. Cycle time is days, not minutes.",
    workflow: [
      {
        name: "Walk the build under test",
        text: "QA opens the staging URL in their browser and pins the Design Mode side panel. As bugs surface, click the affected element.",
      },
      {
        name: "Annotate with structured edits",
        text: 'Drop a comment pin with a description ("misaligned with header"). If you know the fix, make it — change the spacing, the colour, the type size — and let Design Mode log the exact selector + property + before/after value.',
      },
      {
        name: "Capture before / after screenshots",
        text: "The screenshot tool grabs the visible tab; pair with the structured diff so reviewers can see both the artefact and the spec.",
      },
      {
        name: "Export and hand off",
        text: "Use Export → Markdown to paste into Linear, GitHub, or Jira. Developers see selector, property, old value, new value, and the screenshot context in one ticket.",
      },
    ],
    outcome:
      'Developers stop asking "what did you mean?" The bug ticket has the exact change to make. Cycle time drops from days to a single PR review.',
    related: [
      "bug-report-with-visual-diff",
      "design-review-in-production",
      "accessibility-quick-fixes",
    ],
  },
  {
    slug: "bug-report-with-visual-diff",
    persona: "Designers & PMs",
    title: "Bug reports with a visual diff",
    metaTitle:
      "Bug reports with a visual diff — Linear, GitHub, Jira-ready output",
    metaDescription:
      "File bug reports that point at the exact pixel. Design Mode generates a structured diff with selector → property → value that pastes directly into Linear, GitHub, or Jira.",
    keywords: [
      "visual bug report",
      "bug report tool",
      "Linear bug report",
      "Jira bug report",
      "design bug ticket",
      "visual diff",
    ],
    intro:
      "PMs and designers file most of the visual bugs. Engineering has to translate ambiguous descriptions into code changes. Design Mode bridges the two — annotate the bug visually, export a developer-ready diff.",
    problem:
      '"The button is wrong" is not actionable. "The button\'s `padding-block` should change from 8px to 12px and the `background-color` from #3B82F6 to #4F46E5" is. The latter takes 30 seconds in Design Mode.',
    workflow: [
      {
        name: "Open the affected page",
        text: "Browse to the page where the bug appears. Open the Design Mode side panel.",
      },
      {
        name: "Fix it visually",
        text: "Make the change you'd want shipped. Use the contrast checker, the spacing inspector, the type controls.",
      },
      {
        name: "Export markdown diff",
        text: "Click Export → Markdown. The result lists every property change with selectors.",
      },
      {
        name: "Paste into your tracker",
        text: "Linear, GitHub, Jira, Asana — all accept the markdown. Attach the before/after screenshots from Design Mode.",
      },
    ],
    outcome:
      "Developers stop arguing about what the bug actually is and just fix it.",
    related: [
      "ui-testing-export-to-developers",
      "copy-edits-without-a-pr",
      "design-review-in-production",
    ],
  },
  {
    slug: "copy-edits-without-a-pr",
    persona: "Content & marketing teams",
    title: "Copy edits without a PR",
    metaTitle: "Copy edits without a PR — marketing and content team workflow",
    metaDescription:
      "Marketing and content people fix microcopy in the live page with Design Mode, export the structured diff, and hand it to engineering. No Figma round-trip, no Slack screenshot back-and-forth.",
    keywords: [
      "copy editing tool",
      "marketing copy edits",
      "edit website copy",
      "microcopy workflow",
      "content handoff",
    ],
    intro:
      'Marketing teams ship copy changes all day. Every "can we change this headline?" turns into a Slack thread, a Figma comment, and finally a PR — for what amounts to seven words. Design Mode removes every middle step.',
    problem:
      "Content people don't have repo access. Designers don't want to update mocks for microcopy. Engineering interrupts their flow to commit a four-word change.",
    workflow: [
      {
        name: "Edit live copy",
        text: "Open the side panel on the live page or staging URL. Click any text, type the new copy.",
      },
      {
        name: "Export the diff",
        text: "Markdown export captures every text change with selectors.",
      },
      {
        name: "Hand off to engineering",
        text: "One ticket with the diff and a screenshot — engineering does the commit.",
      },
    ],
    outcome:
      "Faster copy iterations, fewer interrupted engineers, no mockup drift.",
    related: [
      "bug-report-with-visual-diff",
      "client-handoff-from-agency-to-engineering",
      "design-review-in-production",
    ],
  },
  {
    slug: "accessibility-quick-fixes",
    persona: "Accessibility & design",
    title: "Accessibility quick fixes",
    metaTitle:
      "Accessibility quick fixes — WCAG contrast, type size, focus states",
    metaDescription:
      "Use Design Mode to fix accessibility issues visually — bump contrast, increase type size, fix focus states — then export the diff for the a11y backlog. WCAG contrast checker built in.",
    keywords: [
      "accessibility audit tool",
      "WCAG contrast checker",
      "a11y fixes",
      "fix contrast issues",
      "accessibility design tool",
    ],
    intro:
      "Most accessibility issues are visual — contrast ratios, small type, missing focus states, low-touch targets. Design Mode catches and fixes all four directly in the browser.",
    problem:
      'Accessibility audit tools list violations. They don\'t help you fix them on the rendered page or generate a ticket developers can act on. The gap between "this fails WCAG 2.2 AA" and "here\'s the change" is the slowest part.',
    workflow: [
      {
        name: "Open the page and walk it",
        text: "Open the side panel. Use the colour picker to spot-check contrast against the WCAG built-in checker.",
      },
      {
        name: "Fix issues visually",
        text: "Bump text size, change colour, tweak the focus ring, expand touch targets. Every fix is a structured change.",
      },
      {
        name: "Export the backlog",
        text: "Markdown export lists each fix with the failing selector. Attach to the a11y backlog or a single PR.",
      },
    ],
    outcome: "An a11y pass that produces shippable code, not just findings.",
    related: [
      "ui-testing-export-to-developers",
      "bug-report-with-visual-diff",
      "design-system-audit",
    ],
  },
  {
    slug: "landing-page-iteration-for-indie-hackers",
    persona: "Indie hackers & solo founders",
    title: "Landing-page iteration for indie hackers",
    metaTitle:
      "Landing-page iteration for indie hackers — Design Mode + Claude Code",
    metaDescription:
      "Solo makers iterate on their landing page: edit the rendered page in Design Mode, hand the diff to Claude Code or Cursor, then review the source change. No Figma, no copy-paste.",
    keywords: [
      "landing page iteration",
      "indie hacker tools",
      "solo founder design",
      "vibe coding landing page",
      "AI agent landing page",
    ],
    intro:
      "If you're a solo founder, the landing page is your conversion funnel and your weekend project. Design Mode + an AI agent is the fastest way to test a hypothesis on the rendered page.",
    problem:
      "Iterating on a landing page in a Figma file, copying decisions back into code, then deploying is a four-step loop. By the time you ship, you've lost the energy of the original idea.",
    workflow: [
      {
        name: "Open the live page",
        text: "Open your deployed landing page. Open the side panel.",
      },
      {
        name: "Iterate visually",
        text: "Try the new hero copy. Try the new CTA colour. Try the spacing variant. Every iteration is a structured change you can undo.",
      },
      {
        name: "Ship via your agent",
        text: "Use Send to Agent with a verified MCP client. The client retrieves the session, implements the source change with its own repository access, and returns a diff for review.",
      },
    ],
    outcome: "You can test a hypothesis on the rendered page, then implement and review it in source. The preview is not a live deploy.",
    related: [
      "redesign-any-website",
      "figma-to-code-without-figma",
      "vibe-coding-with-claude-code",
    ],
  },
  {
    slug: "client-handoff-from-agency-to-engineering",
    persona: "Agencies",
    title: "Client handoff: agency to engineering",
    metaTitle:
      "Client handoff from agency to engineering — structured design specs",
    metaDescription:
      "Agencies use Design Mode to tweak on the client's staging URL during design review and hand engineering a precise, code-ready spec. Replaces Figma comments and Loom recordings.",
    keywords: [
      "agency design handoff",
      "client design review",
      "design spec for engineering",
      "agency workflow",
      "agency tools",
    ],
    intro:
      "Agency design reviews often happen on the client's staging URL with three people on a call. Design Mode turns that ad-hoc conversation into a structured artefact engineering can act on.",
    problem:
      "Agencies own the design; the client's engineering team owns the build. The handoff is the most expensive part of the project — every ambiguity becomes a billable round-trip.",
    workflow: [
      {
        name: "Run the review on staging",
        text: "Pull up the staging URL. Open Design Mode. As decisions land, make the changes in the panel.",
      },
      {
        name: "Bundle the spec",
        text: "Export the Changes tab as JSON + markdown. Attach screenshots.",
      },
      {
        name: "Send to the client's engineers",
        text: "Engineering reads the structured spec and ships. Optional: their coding agent implements the change when it has repository access; someone still reviews the diff.",
      },
    ],
    outcome: "Fewer billable rounds. Cleaner handoff. Happier clients.",
    related: [
      "design-review-in-production",
      "bug-report-with-visual-diff",
      "copy-edits-without-a-pr",
    ],
  },
  {
    slug: "design-system-audit",
    persona: "Design-system maintainers",
    title: "Design-system audit on a deployed app",
    metaTitle:
      "Design-system audit — find token drift in a deployed app with Design Mode",
    metaDescription:
      "Walk a deployed app with Design Mode and log every off-spec colour, spacing, type, or radius. Export the full audit as a structured spec for engineering.",
    keywords: [
      "design system audit",
      "design token drift",
      "design system maintenance",
      "design QA",
      "design ops",
    ],
    intro:
      "Design systems decay. Engineers under deadline reach for raw hex codes; designers hand off without checking tokens; a year later the deployed app uses 14 shades of grey. Design Mode is the fastest way to find and fix the drift.",
    problem:
      "Reading code for drift means parsing thousands of CSS rules. Reading the design system file doesn't tell you what's actually deployed. The only ground truth is the rendered page.",
    workflow: [
      {
        name: "Walk the deployed app",
        text: "Open key pages with Design Mode. Use the colour picker to spot non-token colours.",
      },
      {
        name: "Log drift as structured changes",
        text: "Replace off-spec values with the correct token. Each replacement is logged.",
      },
      {
        name: "Export the audit",
        text: "Markdown export becomes the audit report. Engineering ships the fixes as one or many PRs.",
      },
    ],
    outcome:
      "A deployed app that matches the design system. A reproducible audit that can run again next quarter.",
    related: [
      "tailwind-component-tuning",
      "design-review-in-production",
      "accessibility-quick-fixes",
    ],
  },
];

export function getUseCase(slug: string): UseCase | undefined {
  return useCases.find((u) => u.slug === slug);
}
