export type QA = {
  question: string;
  answer: string;
  /** Optional follow-up links rendered after the answer (not in JSON-LD). */
  links?: { label: string; href: string }[];
};
export type FaqGroup = { title: string; items: QA[] };

export const PRODUCT_DEFINITION =
  "Design Mode is a free, open-source browser extension for Chrome, Firefox and Safari. It lets you edit a webpage visually, records the exact changes, and hands them to a coding agent. The browser edit is a preview; the agent updates source code in a repository it can access.";

export const faqGroups: FaqGroup[] = [
  {
    title: "Product and workflow",
    items: [
      {
        question: "What is Design Mode?",
        answer: PRODUCT_DEFINITION,
      },
      {
        question: "What can I edit?",
        answer:
          "On a scriptable webpage, you can inspect elements and change layout, typography, colour, spacing, effects, motion, text and DOM structure. Design Mode records style, text, DOM and comment changes in the Changes tab. Browser-internal pages, extension stores and other protected surfaces cannot be scripted by browser extensions.",
      },
      {
        question: "Does it work on localhost and sites I'm logged into?",
        answer:
          "Yes on a normal webpage your browser can script — including localhost, staging, and pages where you are already logged in — because the editor runs in that tab. It cannot run on browser-internal pages (chrome://, about:), extension stores, or other blocked surfaces such as some enterprise policies. file:// pages need the browser's local-file permission and are not guaranteed. Cookie-authenticated apps work only while that tab is already signed in; Design Mode does not bypass login, SSO or publisher access controls. Edits are a local preview; they do not change the publisher's source.",
      },
      {
        question: "Does Design Mode change my production source code?",
        answer:
          "No. It only changes what you see in your browser and records those changes. You can copy the notes or send them to a coding agent. The agent still needs access to your project and must make the real code change.",
      },
      {
        question: "Can I edit a third-party website?",
        answer:
          "You can use Design Mode to make a local visual experiment on a webpage that allows extension scripts. That does not change the publisher's website or deploy anything. Reload persistence depends on the browser session and storage implementation described on the privacy page.",
      },
      {
        question: "Who is it for?",
        answer:
          "The primary workflow is for designers, design engineers and developers who use an AI coding agent: make the intended change on the rendered interface, hand the structured change to the agent, then review the source diff and rendered result. QA, content and product teams can also export precise visual feedback without asking the extension to write code.",
      },
    ],
  },
  {
    title: "Browsers and installation",
    items: [
      {
        question: "Which browsers are supported?",
        answer:
          "Design Mode works in Chrome, Firefox and Safari. Features differ a little between them; the browser support and install guides have the details. Chrome and Firefox work right away: install it from the Chrome Web Store or Firefox Add-ons. Safari needs a quick manual setup that takes under 30 seconds.",
        links: [
          { label: "Browser support", href: "/docs/browser-support" },
          { label: "Install guides", href: "/docs" },
        ],
      },
      {
        question: "Should I use Chrome or Firefox?",
        answer:
          "Use Chrome or another Chromium browser if you need pop-out, Picture-in-Picture or the screen eyedropper. Use Firefox if you prefer its native sidebar and do not need those three extras. The core editing surface, Changes tab and MCP hand-off are available in both builds.",
      },
      {
        question: "Is an account required?",
        answer:
          "No account is required to install or use the editor. Connecting the hosted Cloud relay creates an anonymous device credential; it does not ask for an email address or user profile.",
      },
    ],
  },
  {
    title: "Agent hand-off and MCP",
    items: [
      {
        question:
          "How do I communicate visual changes to Claude Code or another coding agent?",
        answer:
          "Use Copy as Prompt when you want a portable Markdown specification that you can paste anywhere. Use Send to Agent when your coding agent is connected through Design Mode's MCP server. In both cases, review the agent's source diff and the rendered result before shipping.",
      },
      {
        question: "What does the MCP server expose?",
        answer:
          "Cloud, Local and Self-hosted expose eight session tools: get_changes, apply_changes, set_change_status, clear_changes, mark_comment_resolved, get_session_summary, export_changes and get_screenshot. Local also exposes wait_for_handoff for opt-in feedback rounds with immutable per-Send snapshots and an explicit Stop. The tools read the current browser session, preview changes in the page and keep the Changes tab in sync; they do not grant an agent repository access. Cloud and Self-hosted keep one-shot Send.",
      },
      {
        question: "Which agents are supported?",
        answer:
          "Design Mode uses Model Context Protocol, so it can work with clients that support a compatible local or Streamable HTTP MCP server. Configuration files, transport support and authentication syntax differ by client and version. Use the tested, version-stamped instructions in the MCP setup documentation instead of assuming one JSON block works everywhere.",
      },
      {
        question:
          "What is the difference between Cloud, Local and Self-hosted?",
        answer:
          "They are three ways to pass your changes to your AI assistant. Cloud is the easiest. Your changes travel through our secure online service, and nothing connects until you turn it on. Local keeps everything on your own computer. Self-hosted is for teams that want to run that service themselves.",
      },
      {
        question: "Do I have to use MCP?",
        answer:
          "No. Copy as Prompt works without an MCP server. MCP is useful when you want the agent to fetch changes, return live previews, capture screenshots and update change status directly.",
      },
    ],
  },
  {
    title: "Privacy, storage and licence",
    items: [
      {
        question: "What data leaves my machine by default?",
        answer:
          "Release builds of the extension send bounded feature usage events to PostHog with a random installation ID so we can see which features work and which fail — never page URLs, text, selectors, comments or screenshots. A fresh install selects Cloud mode for MCP but does not contact the relay until you create a Cloud credential. The marketing website may load Google Analytics when its deployment ID is configured. See the privacy page for full storage, relay and analytics details.",
      },
      {
        question: "How does the Cloud relay store data?",
        answer:
          "Relay request and response queues request a 60-second Redis expiry and responses are normally deleted as soon as they are consumed. The expiry call is separate and best-effort, so 60 seconds is not a guaranteed maximum. The server retains a SHA-256 hash of the anonymous device token, a tenant identifier and created/last-seen timestamps until the credential is revoked. Operational logs contain metadata such as tenant identifier, message type, byte count, latency, status and errors—not selectors, screenshots or payload bodies.",
      },
      {
        question: "Does Design Mode train on my edits?",
        answer:
          "Design Mode does not train a model and its relay does not send edits to a model on its own. When you deliberately hand changes to Claude, Cursor or another agent, that provider processes the content under its own account settings and privacy terms.",
      },
      {
        question: "Is Design Mode free and open source?",
        answer:
          "The extension and repository are published under the MIT licence and the current store listings are free to install. MIT permits commercial use, modification and redistribution subject to its licence notice. Future hosting or service policy should be judged from the terms published at that time rather than a promise that can never change.",
      },
    ],
  },
  {
    title: "Support",
    items: [
      {
        question: "How do I report a bug?",
        answer:
          "Open Help in the extension, choose Copy diagnostics, and add the result with reproduction steps to a GitHub issue. Do not include passwords, tokens, private page content or confidential screenshots.",
      },
      {
        question: "How do I report a security issue?",
        answer:
          "Follow SECURITY.md in the GitHub repository. Use the private disclosure channel described there rather than publishing exploit details in a public issue.",
      },
    ],
  },
];

const homepageQuestions = new Set([
  "What is Design Mode?",
  "Does Design Mode change my production source code?",
  "Which browsers are supported?",
  "How do I communicate visual changes to Claude Code or another coding agent?",
  "What is the difference between Cloud, Local and Self-hosted?",
  "What data leaves my machine by default?",
  "Is Design Mode free and open source?",
]);

export const homepageFaqQA = faqGroups
  .flatMap((group) => group.items)
  .filter((item) => homepageQuestions.has(item.question));
