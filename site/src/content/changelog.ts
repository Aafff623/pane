// Designer-friendly, dated release highlights for the public /changelog page.
// Plain language on purpose — this is the "what shipped in this version"
// record (mirrors released versions in CHANGELOG.md, not an inventory of
// every change). Do not add Unreleased items as if they have shipped.
//
// Newest release first. Each `date` is an ISO YYYY-MM-DD so it renders in a
// <time datetime> element. Keep entries user-facing — skip pure internal /
// build / website-infra changes.

export type Release = {
  version: string;
  /** ISO date, e.g. "2026-08-18". */
  date: string;
  /** One-line headline for the release. */
  headline: string;
  /** Designer-facing "what you can do now" bullets. */
  highlights: string[];
};

export const releases: Release[] = [
  {
    version: "3.1.0",
    date: "2026-10-16",
    headline: "Clearer usage analytics and a homepage demo.",
    highlights: [
      "Usage analytics now records which CSS property and design section you edit, which text or DOM action you use, and a bounded reason when a command fails. Page content, CSS values and URLs stay out.",
      "The homepage plays a product demo, and the secondary button now says Try it yourself.",
      "Homepage sections use less vertical space on phones.",
    ],
  },
  {
    version: "3.0.2",
    date: "2026-10-15",
    headline: "Browser-aware banner, four new comparisons, and expanded usage analytics.",
    highlights: [
      "The top banner now adapts to your browser and links to the other two supported browsers (Chrome, Firefox, or Safari).",
      "Added four source-backed comparison pages: Design Mode vs EditUI, Airship, LayerDock, and Vex.",
      "Fixed Open Graph and Twitter social preview cards across the website.",
      "Expanded bounded extension usage analytics (schema v3) and updated the privacy policy; removed the redundant Settings notice.",
      "Rate-limited Cloud credential registration and refreshed production dependencies.",
    ],
  },
  {
    version: "3.0.1",
    date: "2026-10-06",
    headline: "Clearer install options for Chrome, Firefox and Safari.",
    highlights: [
      "The install button now matches your browser: Add to Chrome, Add to Firefox, or Download latest on Safari.",
      "The site banner links straight to Firefox Add-ons and the Safari download.",
      "Simpler answers in the FAQ about supported browsers and connection modes.",
    ],
  },
  {
    version: "3.0.0",
    date: "2026-10-03",
    headline: "Safari, cloud connections and a CLI.",
    highlights: [
      "Use Design Mode in Safari on Mac as a Web Inspector tab. Install the Safari ZIP from GitHub Releases as a temporary extension; Safari removes it after 24 hours or when it quits.",
      "Connect a coding agent through the hosted relay or your own Redis relay, with no local companion running.",
      "Set up Cloud, Local or Self-hosted MCP from the terminal with the new designmode-app CLI.",
      "Send to Agent, Copy as Prompt and JSON export can include saved routes for the whole origin, not just the current page.",
      "Saved edits now survive a reload in Firefox, and element comments stay attached to the element you left them on.",
      "Release builds send a small set of anonymous feature events so we can see which features are used. No page content, selectors, comments or screenshots are sent.",
    ],
  },
  {
    version: "2.3.1",
    date: "2026-09-20",
    headline: "A smoother homepage and clearer demo guidance.",
    highlights: [
      "Watch the coding-agent icons keep crossfading in a continuous loop while the hero title scales smoothly across screen sizes.",
      "On mobile, the guided demo now directs you to the desktop extension experience instead of showing steps that cannot be completed there.",
    ],
  },
  {
    version: "2.3.0",
    date: "2026-09-20",
    headline: "Keep design intent intact and iterate with your agent.",
    highlights: [
      "Run opt-in Local feedback rounds: Send each reviewed snapshot to the agent, see Waiting / Implementing state, and stop the loop without clearing your edits.",
      "See authored width and height separately from computed pixels, so responsive units, intrinsic sizes, expressions, and tokens stay honest.",
      "Group Changes by detected component/source context while keeping every selector and per-element action available.",
      "Edit link destinations from a Markdown-style URL row while preserving hidden accessibility text, icons, media, and attributed child spans.",
      "Delete a Shift-selected Layers set in one action, with one ordered, reversible Changes entry per layer.",
      "Set up and diagnose the Local companion with guided, preview-first setup for Claude Code and Cursor.",
    ],
  },
  {
    version: "2.2.1",
    date: "2026-09-05",
    headline: "Launch where you left off, even after Chrome sleeps.",
    highlights: [
      "Alt+D preserves the side-panel launch gesture after a service-worker cold start.",
      "Floating and Pin on top restore without briefly opening the docked panel first.",
    ],
  },
  {
    version: "2.2.0",
    date: "2026-09-05",
    headline: "Inspect real states and keep every agent connected.",
    highlights: [
      "Inspect hover, focus, focus-visible, and active styles already authored on the page, then force a state to preview it while you edit.",
      "See how flex and grid containers are really laid out with an optional computed overlay of tracks and children.",
      "Choose where Design Mode opens in Chrome: the docked side panel, a floating window, or a Pin-on-top launcher.",
      "Use Cursor, Claude Desktop, and other local MCP clients together: they share one owner on port 9960 and a surviving client takes over if the owner closes.",
      "Polish and reliability: readable dark-theme dropdowns, whole-word text diffs, media-safe rich-text edits, Enter-to-commit fields, and dependable browser screenshot shortcuts.",
    ],
  },
  {
    version: "2.1.0",
    date: "2026-08-18",
    headline: "Corner shapes and responsive-aware edits.",
    highlights: [
      "Corner shape — round your corners into a squircle (the smooth iOS look), a bevel, a scoop, or a notch, straight from a dropdown next to corner radius. No more settling for plain rounding.",
      "Responsive tags on edits — every change remembers the screen size you made it at (mobile, tablet, or desktop) and labels it, so responsive tweaks are obvious and your AI agent knows to wrap them in the right media query.",
      "A tidier Design panel — sections that don't apply to what you selected (no stroke, no effects, no fill) now fold away, so you only see the controls that matter.",
      "A friendly heads-up when your browser's device preview turns hovering off, with a one-tap way to keep working.",
      "Polish: cleaner corner-radius values, and copy / download buttons that confirm with a checkmark.",
    ],
  },
  {
    version: "2.0.0",
    date: "2026-07-30",
    headline: "Firefox support and snap-to-align.",
    highlights: [
      "Firefox support — Design Mode now runs in Firefox as well as Chrome-based browsers, from the same extension.",
      "Snap-to-align while dragging — move an element and it snaps into line with its neighbours, with magenta guides just like Slides, Keynote, or Figma.",
      "A simpler color picker — opens to the essentials (swatch, hex, eyedropper) with the full color wheel one click away.",
      "A refreshed dark theme, plus a batch of reliability fixes (undo/redo, Escape back to hover, multi-select).",
    ],
  },
  {
    version: "1.9.0",
    date: "2026-07-19",
    headline: "Design-system aware, with visual motion.",
    highlights: [
      "Design-system aware — Design Mode finds the design tokens on a page (colors, spacing, type), even themed ones, labels the system (Carbon, Material, MUI, Bootstrap, Polaris, Radix, shadcn/ui, Tailwind), and lets you swap or edit a token once to update everything that uses it.",
      "Motion made visual — add hover, press, focus, appear, loop, or scroll animations from ready-made presets, each with a live preview.",
      "A smoother hand-off to your coding agent — a dedicated MCP page and a step-by-step Send-to-Agent flow.",
    ],
  },
  {
    version: "1.8.0",
    date: "2026-07-08",
    headline: "Float the panel, edit local files.",
    highlights: [
      "Pin on top — float the panel above everything as an always-on-top window while you work in another app.",
      "Edit local files — open an HTML file straight from your computer and edit it just like a live site.",
      "A custom page cursor, and per-page edit sessions that stick around when you come back.",
    ],
  },
  {
    version: "1.7.1",
    date: "2026-07-03",
    headline: "Polish and fixes.",
    highlights: [
      "Auto gap no longer disturbs how children line up, plus small website touch-ups.",
    ],
  },
  {
    version: "1.7.0",
    date: "2026-06-02",
    headline: "Region comments and a pop-out panel.",
    highlights: [
      "Region comments — draw a box anywhere on the page to leave feedback on an area, not just a single element.",
      "Pop-out window — detach the side panel into a floating window you can move around.",
      "Real spacing controls — dedicated Margin and Padding rows, fixed or auto gap, and adjustable nudge steps.",
      "A shortcuts popover and cleaner screenshots.",
    ],
  },
  {
    version: "1.6.0",
    date: "2026-05-29",
    headline: "Tokens panel and Copy as Prompt.",
    highlights: [
      "Design-system / Tokens panel — a home for your saved styles and the page's design tokens.",
      "Copy as Prompt — export every change you've made as a clean, ready-to-paste brief for your AI coding agent.",
    ],
  },
  {
    version: "1.5.0",
    date: "2026-05-23",
    headline: "Spacing overlays and a contrast checker.",
    highlights: [
      "See spacing at a glance — margin and padding render as coloured bands on the element.",
      "A WCAG contrast checker built right into the color picker.",
      "Configurable inspector overlay colors, and file size shown for images and media.",
    ],
  },
  {
    version: "1.4.0",
    date: "2026-05-23",
    headline: "Measure, resize, and move on the page.",
    highlights: [
      "Measure & resize — VisBug-style measurement guides, eight drag handles to resize, drag-to-move, and shift-click to select several elements at once, all on the live page.",
    ],
  },
  {
    version: "1.3.0",
    date: "2026-05-20",
    headline: "Live agent presence.",
    highlights: [
      "The panel now shows when your coding agent is actually connected, so you know Send-to-Agent is ready.",
      "Cloud MCP became the default for fresh installs — no local setup required to start.",
    ],
  },
  {
    version: "1.2.0",
    date: "2026-05-19",
    headline: "Contribute panel.",
    highlights: [
      "A Contribute panel in the extension, plus canonical-URL and diagnostics polish.",
    ],
  },
  {
    version: "1.1.0",
    date: "2026-05-14",
    headline: "In-panel Help and quick tab switching.",
    highlights: [
      "An in-panel Help view, and Alt+1 / Alt+2 / Alt+3 to jump between the Design, Layers, and Changes tabs.",
    ],
  },
  {
    version: "1.0.2",
    date: "2026-05-12",
    headline: "Stroke, Effects, Motion, and Layout guides.",
    highlights: [
      "A big Design-tab expansion — Stroke, Effects (including Noise and Texture), Motion, and Layout-guide sections, plus reusable style presets so you can save a look and reapply it.",
    ],
  },
  {
    version: "1.0.1",
    date: "2026-05-04",
    headline: "First public release.",
    highlights: [
      "The first public release — inspect elements on a scriptable webpage and edit layout, type, colour, and spacing with visual controls, then hand the diff to a coding agent that has repository access.",
    ],
  },
];
