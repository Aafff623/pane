// One source of truth for "how does this provider connect?".
//
// The Customize drawer used to answer that question with catalog flags
// alone, and the flags cannot express the difference between an API-key
// provider and a scan/browser-login provider — so every login-only family
// fell through to the paste-key dialog (12 families, 11 of which really
// stored a key that then failed as `unknown provider`). This table is the
// missing vocabulary: the credential shape, the actions that can actually
// complete it, and the hint/steps the user needs to follow.
//
// Defaults are derived from the catalog; `SPECIAL` overrides the families
// whose real flow differs from the flag. Keep `actions` honest: a family
// with no API key must never expose `pasteKey`.

import { providerCatalog, supportsApiKey } from "./providerCatalog";

export type CredentialKind =
  | "apiKey"
  | "oauthDevice"
  | "oauthBrowser"
  | "qrScan"
  | "localRead"
  | "cookie"
  | "cloud"
  | "none";

export type AccountAction =
  | "addLogin"
  | "pasteKey"
  | "captureSlots"
  | "importAccounts"
  | "importCookie"
  | "openAuthCenter"
  | "pasteKeyPage";

export interface Onboarding {
  kind: CredentialKind;
  /** Ordered actions the user can take right now. */
  actions: AccountAction[];
  /** One line: what this provider reads. */
  hintKey: string;
  /** Optional numbered steps for a login flow. */
  stepsKey?: string;
}

const OAUTH_DEVICE = ["codex", "grok", "copilot"] as const;
const OAUTH_BROWSER = ["kiro", "windsurf", "trae", "zed", "qoder"] as const;
const QR_SCAN = ["codebuddy", "codebuddycn", "workbuddy", "workbuddyai"] as const;
/** Search/MCP keys live in search-keys.json, not the provider key store. */
const SEARCH_KEYS = ["bocha", "tavily", "firecrawl", "brave"] as const;

const LOCAL_HINTS: Record<string, string> = {
  orcaterm: "customize.loginHint.orcaterm",
  claude: "customize.loginHint.claude",
  devin: "customize.loginHint.devin",
  ollama: "customize.loginHint.ollama",
  hermes: "customize.loginHint.hermes",
  shandianshuo: "customize.loginHint.shandianshuo",
  qodercn: "customize.loginHint.qodercn",
  traecn: "customize.loginHint.traecn",
  jetbrains: "customize.loginHint.jetbrains",
  zcode: "customize.loginHint.zcode",
};

const SPECIAL: Record<string, Onboarding> = {
  // Browser/device-code logins owned by Pane's auth center.
  ...Object.fromEntries(
    OAUTH_DEVICE.map((family) => [
      family,
      { kind: "oauthDevice" as CredentialKind, actions: ["addLogin" as AccountAction], hintKey: `customize.loginHint.${family}` },
    ]),
  ),
  ...Object.fromEntries(
    OAUTH_BROWSER.map((family) => [
      family,
      {
        kind: "oauthBrowser" as CredentialKind,
        actions: ["addLogin" as AccountAction],
        hintKey: `customize.loginHint.${family}`,
        stepsKey: `customize.onboarding.${family}`,
      },
    ]),
  ),
  ...Object.fromEntries(
    QR_SCAN.map((family) => [
      family,
      {
        kind: "qrScan" as CredentialKind,
        actions: ["addLogin" as AccountAction],
        hintKey: `customize.loginHint.${family}`,
        stepsKey: `customize.onboarding.${family}`,
      },
    ]),
  ),
  cursor: {
    kind: "localRead",
    actions: ["importAccounts"],
    hintKey: "customize.loginHint.cursor",
  },
  antigravity: {
    kind: "localRead",
    actions: ["captureSlots"],
    hintKey: "customize.loginHint.antigravity",
  },
  onenewapi: {
    kind: "localRead",
    actions: [],
    hintKey: "customize.onboarding.onenewapi",
  },
  stepfun_plan: {
    kind: "localRead",
    actions: [],
    hintKey: "customize.onboarding.stepfunPlan",
  },
  "stepfun-plan": {
    kind: "localRead",
    actions: [],
    hintKey: "customize.onboarding.stepfunPlan",
  },
  sensenova: {
    kind: "oauthBrowser",
    actions: ["addLogin"],
    hintKey: "customize.onboarding.sensenova",
  },
  doubao: {
    kind: "cookie",
    actions: [],
    hintKey: "customize.loginHint.doubao",
  },
  tokenrhythm: {
    kind: "cookie",
    actions: ["pasteKeyPage"],
    hintKey: "customize.loginHint.tokenrhythm",
    stepsKey: "customize.onboarding.tokenrhythm",
  },
  xfyun: {
    kind: "cookie",
    actions: ["addLogin"],
    hintKey: "customize.loginHint.xfyun",
    stepsKey: "customize.onboarding.xfyun",
  },
  "xfyun-speech": {
    kind: "cookie",
    actions: ["addLogin"],
    hintKey: "customize.loginHint.xfyun-speech",
    stepsKey: "customize.onboarding.xfyun-speech",
  },
  bedrock: {
    kind: "cloud",
    actions: [],
    hintKey: "customize.loginHint.bedrock",
  },
  vertexai: {
    kind: "cloud",
    actions: [],
    hintKey: "customize.loginHint.vertexai",
  },
  giphy: {
    kind: "apiKey",
    actions: ["pasteKey"],
    hintKey: "customize.onboarding.giphy",
  },
};

/** WorkBuddy's extra web-session surface. */
SPECIAL.workbuddy.actions = ["addLogin", "importCookie"];
SPECIAL.workbuddyai.actions = ["addLogin"];

const SEARCH_SPECIAL: Record<string, Onboarding> = Object.fromEntries(
  SEARCH_KEYS.map((family) => [
    family,
    {
      kind: "apiKey" as CredentialKind,
      actions: ["pasteKeyPage" as AccountAction],
      hintKey: `customize.loginHint.${family}`,
    },
  ]),
);

const DERIVED: Record<string, Onboarding> = Object.fromEntries(
  providerCatalog.map((definition) => {
    const family = definition.familyId;
    if (SPECIAL[family]) return [family, SPECIAL[family]];
    if (SEARCH_SPECIAL[family]) return [family, SEARCH_SPECIAL[family]];
    if (LOCAL_HINTS[family]) {
      return [
        family,
        { kind: "localRead" as CredentialKind, actions: [], hintKey: LOCAL_HINTS[family] },
      ];
    }
    if (supportsApiKey(family)) {
      return [
        family,
        {
          kind: "apiKey" as CredentialKind,
          actions: ["pasteKey" as AccountAction],
          hintKey: "customize.onboarding.apiKey",
        },
      ];
    }
    return [
      family,
      {
        kind: "localRead" as CredentialKind,
        actions: [],
        hintKey: "customize.onboarding.localRead",
      },
    ];
  }),
);

export function onboardingFor(family: string): Onboarding {
  return (
    DERIVED[family] ?? {
      kind: "localRead",
      actions: [],
      hintKey: "customize.onboarding.localRead",
    }
  );
}

/** The "?" panel's method list, derived from the same descriptor. */
export function onboardingMethods(family: string): Array<"paste" | "oauth" | "local"> {
  const kind = onboardingFor(family).kind;
  switch (kind) {
    case "apiKey":
      return ["paste"];
    case "oauthDevice":
    case "oauthBrowser":
    case "qrScan":
      return ["oauth"];
    case "localRead":
    case "cookie":
    case "cloud":
      return ["local"];
    case "none":
      return [];
  }
}
