export const DEMO_GUIDE = ["overview", "header", "actions", "accounts", "spend", "settings", "shortcuts"] as const;
export type DemoGuide = (typeof DEMO_GUIDE)[number];
