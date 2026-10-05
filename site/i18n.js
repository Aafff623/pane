// The home page in other languages. public/index.html is written in
// Chinese (Pane's primary audience), each string to translate marked
// data-i18n="key" (its inner HTML) or data-i18n-attr="attr:key,..." (its
// attributes); worker.js serves the page at /<lang>/ with these put in.
// site.test.mjs fails on a key the page doesn't have or the dictionary
// misses — the two stay in step by force, not by memory.
//
// A language is added here and in LANGS (its dictionary and its
// "<html lang>" value), in the page's language menu, and in any script's
// T() calls. window.T in index.html takes the Chinese first, then one
// argument per language in LANGS order; the test asserts the count.

const en = {
  "meta.desc": "Pane is a Windows tray panel that keeps the quotas, balances and spend of 41 AI coding tools and model vendors in one place. Keys stay on your machine; queries go straight to each vendor's own API.",
  "meta.tagline": "Every AI tool's quota and spend, at a glance.",

  "nav.how": "How it works",
  "nav.providers": "Providers",
  "nav.shots": "Screenshots",
  "nav.api": "Local API",
  "nav.releases": "Releases",
  "nav.language": "Language",
  "nav.menu": "Menu",
  "nav.download": "Download",

  "hero.h1": "Every AI tool's quota and spend, <span>at a glance.</span>",
  "hero.lede": "Pane is a local-first panel that lives in the Windows tray, gathering the quotas, balances and subscription states of the AI coding tools and model vendors you use — Claude, Codex, Cursor, Kimi and 41 in all. Keys stay on your machine, queries go straight to each vendor's own API, and your data never leaves your computer.",
  "hero.dl": "Download the latest release",
  "hero.gh": "GitHub repository",
  "hero.free": "Free",
  "hero.oss": "Open source (MIT)",
  "hero.os": "Windows-first; macOS and Linux builds ship with each release",

  "alt.overview": "The Pane panel: quota progress and reset times for each AI tool",
  "alt.spend": "Pane's spend panel: tokens and cost grouped by tool and model",
  "alt.heatmap": "Pane's 365-day daily spend heatmap",

  "prov.eyebrow": "Providers",
  "prov.h2": "41 vendors, one wall.",
  "prov.sub": "Model vendors, relay services and local servers, plus custom balance queries — from Claude, Codex and Cursor to DeepSeek, Kimi and SiliconFlow, with the list growing every release.",

  "how.eyebrow": "How it works",
  "how.h2": "Three steps, all on your machine.",
  "how.sub": "No accounts, no cloud sync. Pane talks only to your computer and to each vendor's own API.",
  "how.s1.t": "Add credentials",
  "how.s1.p": "Add API keys or OAuth logins to Pane. Keys are written only to Pane's local data directory, and an optional master-password vault locks them further.",
  "how.s2.t": "Query locally",
  "how.s2.p": "Pane polls each vendor's own API for quotas and balances, and scans local CLI logs for spend — never through a third-party server.",
  "how.s3.t": "One panel",
  "how.s3.p": "The tray icon projects the tightest quota in real time; open the panel for per-vendor progress, reset countdowns and the spend heatmap.",

  "det.eyebrow": "Details",
  "det.h2": "Local-first, taken seriously.",
  "det.local.t": "Keys never leave the machine",
  "det.local.p": "Tokens go only to their own vendor's API. The local endpoint binds to loopback, checks the Host header and has no CORS.",
  "det.vault.t": "Master-password vault",
  "det.vault.p": "An optional encrypted vault under a master password adds another lock over locally stored credentials, with notes per key.",
  "det.spend.t": "Spend stats and heatmap",
  "det.spend.p": "Tokens and cost grouped by tool and by model, with a daily heatmap reaching back 365 days.",
  "det.tray.t": "Lives in the tray",
  "det.tray.p": "Rests quietly in the Windows tray, with optional toasts when a quota runs low. It does not nag.",
  "det.update.t": "Signed auto-updates",
  "det.update.p": "New versions are downloaded from GitHub Releases, signature-checked and installed — no website visit needed.",
  "det.api.t": "Local JSON API",
  "det.api.p": "A read-only endpoint hands the panel's data to your own scripts and automation, below.",

  "shot.eyebrow": "Screenshots",
  "shot.h2": "The interface, as it is.",
  "shot.sub": "One shot per theme with fixed file names — replace the files of the same name under site/public/img/ to update.",
  "shot.spend.cap": "Spend by tool and by model",
  "shot.heat.cap": "365 days of daily spend",
  "shot.note": "These are placeholders: swap overview / spend / heatmap -dark.png and -light.png in place — no code change needed.",

  "api.eyebrow": "Local API",
  "api.h2": "The numbers, for your scripts.",
  "api.sub": "Pane exposes a read-only JSON endpoint on your machine that only answers loopback requests. Everything the panel shows, your scripts can read.",
  "api.c1": "# the newest snapshot of every provider",
  "api.c2": "# a single provider",
  "api.note": "The endpoint binds to 127.0.0.1 only, checks the Host header and sends no CORS headers; keys never appear in any response.",
  "copy": "Copy",

  "fin.h2": "Fewer tabs to check. One glance at the tray.",
  "fin.sub": "Free, open source, local-first. Windows-first; macOS and Linux builds ship with each release.",
  "fin.dl": "Download the latest release",
  "fin.gh": "GitHub repository",

  "foot.tag": "Pane — the local-first AI quota panel",
  "foot.src": "Source",
  "foot.rel": "Releases",
  "foot.issues": "Issues",
};

export const LANGS = {
  en: { dict: en, html: "en" },
};
