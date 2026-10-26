// Shortcut kernel semantics adapted from ZCode's shared command table and
// pure bindings/conflicts modules (Apache-2.0, Copyright 2025 ZCode
// contributors). Rewritten for Pane's smaller command set; no UI or DOM in
// this file so every branch is testable with node --test.

export type ShortcutPlatform = "mac" | "windows";
export type ShortcutChannel = "key" | "tap" | "system";
export type ShortcutScope = "dashboard" | "global";

export interface ShortcutCommand {
  id: string;
  labelKey: string;
  defaultBindings: string[];
  channel: ShortcutChannel;
  scope: ShortcutScope;
  /// false = internal command, hidden from Settings (panel jump).
  configurable: boolean;
  /// Older single-value config fields that still own this command.
  legacyField?: "shortcut" | "categoryShortcut";
}

export const SHORTCUT_COMMANDS: readonly ShortcutCommand[] = [
  {
    id: "wake",
    labelKey: "settings.wakeShortcut",
    defaultBindings: ["Alt+2"],
    channel: "system",
    scope: "global",
    configurable: true,
    legacyField: "shortcut",
  },
  {
    id: "category",
    labelKey: "settings.categoryShortcut",
    defaultBindings: ["Shift+1"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
    legacyField: "categoryShortcut",
  },
  {
    id: "settings",
    labelKey: "settings.actionSettings",
    defaultBindings: ["Ctrl+S"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "refresh",
    labelKey: "settings.actionRefresh",
    defaultBindings: ["Ctrl+R"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "customize",
    labelKey: "settings.actionCustomize",
    defaultBindings: ["Ctrl+E"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "theme",
    labelKey: "settings.actionTheme",
    defaultBindings: ["Ctrl+L"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "expiring",
    labelKey: "settings.actionExpiring",
    defaultBindings: ["T"],
    channel: "key",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "period",
    labelKey: "settings.actionPeriod",
    defaultBindings: ["Shift"],
    channel: "tap",
    scope: "dashboard",
    configurable: true,
  },
  {
    id: "panel",
    labelKey: "",
    defaultBindings: ["Ctrl+Shift+P"],
    channel: "key",
    scope: "dashboard",
    configurable: false,
  },
];

export interface LegacyShortcutFields {
  shortcut?: string;
  categoryShortcut?: string;
}

const MODIFIER_ALIASES: Record<string, "ctrl" | "alt" | "shift" | "meta"> = {
  ctrl: "ctrl",
  control: "ctrl",
  cmdorctrl: "ctrl",
  cmd: "meta",
  command: "meta",
  meta: "meta",
  super: "meta",
  win: "meta",
  alt: "alt",
  option: "alt",
  opt: "alt",
  shift: "shift",
};

const MODIFIER_ORDER = ["ctrl", "alt", "shift", "meta"] as const;

export function currentShortcutPlatform(): ShortcutPlatform {
  try {
    return /mac|iphone|ipad/i.test(navigator.platform) ? "mac" : "windows";
  } catch {
    return "windows";
  }
}

/// Physical base key from the layout-independent `code`, with `key` as the
/// fallback for IME, media keys and synthetic events.
export function eventKeyToken(event: Pick<KeyboardEvent, "code" | "key">): string {
  const code = event.code ?? "";
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return code.slice(6);
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^F[0-9]{1,2}$/.test(code)) return code;
  const key = event.key ?? "";
  return key.length === 1 ? key.toUpperCase() : key.toUpperCase();
}

function normalizeKeyToken(token: string): string {
  const trimmed = token.trim();
  if (trimmed.length === 1) return trimmed.toUpperCase();
  if (/^f[0-9]{1,2}$/i.test(trimmed)) return trimmed.toUpperCase();
  return trimmed.toUpperCase();
}

/// Canonical form: modifiers in Ctrl/Alt/Shift/Meta order plus an uppercase
/// key. `CmdOrCtrl` resolves per platform, so the same text cannot match both
/// Ctrl and ⌘ by accident. Null means the text is not a usable binding.
export function canonicalBinding(
  binding: string,
  platform: ShortcutPlatform,
): string | null {
  const parts = binding.split("+").map((part) => part.trim()).filter(Boolean);
  if (!parts.length) return null;
  const key = normalizeKeyToken(parts.pop()!);
  if (!key) return null;
  const modifiers = new Set<string>();
  for (const part of parts) {
    let alias = MODIFIER_ALIASES[part.toLowerCase()];
    if (!alias) return null;
    if (part.toLowerCase() === "cmdorctrl") alias = platform === "mac" ? "meta" : "ctrl";
    modifiers.add(alias);
  }
  return [
    ...MODIFIER_ORDER.filter((modifier) => modifiers.has(modifier)).map((modifier) => modifier.toUpperCase()),
    key,
  ].join("+");
}

export function isImeNoise(event: {
  isComposing?: boolean;
  keyCode?: number;
  repeat?: boolean;
}): boolean {
  return Boolean(event.isComposing) || event.keyCode === 229 || Boolean(event.repeat);
}

const MODIFIER_KEYS = new Set(["Control", "ControlLeft", "ControlRight", "Alt", "AltLeft", "AltRight", "Shift", "ShiftLeft", "ShiftRight", "Meta", "MetaLeft", "MetaRight"]);

export function isModifierOnly(event: Pick<KeyboardEvent, "key">): boolean {
  return MODIFIER_KEYS.has(event.key);
}

/// The binding a real key event represents, or null for modifier-only and
/// IME-composition noise. This is the recorder's source of truth.
export function eventBinding(
  event: KeyboardEvent,
  platform: ShortcutPlatform,
): string | null {
  if (isModifierOnly(event) || isImeNoise(event)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Meta");
  parts.push(eventKeyToken(event));
  return canonicalBinding(parts.join("+"), platform);
}

export function matchesBinding(
  event: KeyboardEvent,
  binding: string,
  platform: ShortcutPlatform,
): boolean {
  if (isImeNoise(event)) return false;
  const expected = canonicalBinding(binding, platform);
  const actual = eventBinding(event, platform);
  return Boolean(expected && actual && expected === actual);
}

export function effectiveBindings(
  commandId: string,
  overrides: Record<string, string[]> | undefined,
  legacy: LegacyShortcutFields = {},
  commands: readonly ShortcutCommand[] = SHORTCUT_COMMANDS,
): string[] {
  const command = commands.find((entry) => entry.id === commandId);
  if (!command) return [];
  if (command.legacyField) {
    const value = legacy[command.legacyField];
    if (value === undefined) return [...command.defaultBindings];
    return value.trim() ? [value.trim()] : [];
  }
  if (overrides && Object.prototype.hasOwnProperty.call(overrides, commandId)) {
    return [...overrides[commandId]];
  }
  return [...command.defaultBindings];
}

/// Explicit empty array survives: a cleared row must not fall back silently.
export function migrateLegacyShortcuts(
  legacy: Record<string, string> | undefined,
  commands: readonly ShortcutCommand[] = SHORTCUT_COMMANDS,
): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  if (!legacy) return out;
  for (const command of commands) {
    if (command.legacyField) continue;
    if (!Object.prototype.hasOwnProperty.call(legacy, command.id)) continue;
    const value = (legacy[command.id] ?? "").trim();
    out[command.id] = value ? [value] : [];
  }
  return out;
}

const RESERVED_KEYS = new Set([
  "ESCAPE", "TAB", "ENTER", "ARROWUP", "ARROWDOWN", "ARROWLEFT", "ARROWRIGHT",
  "HOME", "END", "PAGEUP", "PAGEDOWN", "BACKSPACE", "DELETE", "INSERT",
  "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
]);

const RESERVED_COMBOS = new Set([
  "CTRL+C", "CTRL+V", "CTRL+X", "CTRL+A", "CTRL+Z", "CTRL+Y",
  "CTRL+SHIFT+I", "CTRL+SHIFT+J", "CTRL+SHIFT+C", "CTRL+W",
  "META+C", "META+V", "META+X", "META+A", "META+Z", "META+SHIFT+Z",
]);

export function reservedBinding(binding: string, platform: ShortcutPlatform): boolean {
  const canonical = canonicalBinding(binding, platform);
  if (!canonical) return true;
  const parts = canonical.split("+");
  const key = parts[parts.length - 1];
  const hasModifier = parts.length > 1;
  if (!hasModifier && RESERVED_KEYS.has(key)) return true;
  return RESERVED_COMBOS.has(canonical);
}

export interface RecordedBinding {
  binding: string;
  bare: boolean;
  reserved: boolean;
}

export function recordBinding(
  event: KeyboardEvent,
  platform: ShortcutPlatform,
): RecordedBinding | null {
  if (isModifierOnly(event) || isImeNoise(event)) return null;
  const binding = eventBinding(event, platform);
  if (!binding) return null;
  const bare = !(event.ctrlKey || event.altKey || event.metaKey);
  return { binding, bare, reserved: reservedBinding(binding, platform) };
}

/// The first other command already owning this binding. Legacy/system
/// commands are included so a user cannot silently steal Alt+2.
export function findBindingConflict(
  targetId: string,
  binding: string,
  overrides: Record<string, string[]> | undefined,
  legacy: LegacyShortcutFields,
  platform: ShortcutPlatform,
  commands: readonly ShortcutCommand[] = SHORTCUT_COMMANDS,
): ShortcutCommand | null {
  const canonical = canonicalBinding(binding, platform);
  if (!canonical) return null;
  for (const command of commands) {
    if (command.id === targetId) continue;
    const owned = effectiveBindings(command.id, overrides, legacy, commands);
    if (owned.some((entry) => canonicalBinding(entry, platform) === canonical)) return command;
  }
  return null;
}

/// Takeover keeps every other binding of the losing command (it may end up
/// empty, which stays cleared thanks to the explicit-array rule).
export function takeoverBinding(
  targetId: string,
  binding: string,
  overrides: Record<string, string[]> | undefined,
  platform: ShortcutPlatform,
  commands: readonly ShortcutCommand[] = SHORTCUT_COMMANDS,
): Record<string, string[]> {
  const canonical = canonicalBinding(binding, platform);
  const next: Record<string, string[]> = { ...(overrides ?? {}) };
  if (!canonical) return next;
  for (const command of commands) {
    if (command.id === targetId || command.legacyField) continue;
    const current = effectiveBindings(command.id, next, {}, commands);
    const remaining = current.filter(
      (entry) => canonicalBinding(entry, platform) !== canonical,
    );
    if (remaining.length !== current.length) next[command.id] = remaining;
  }
  return next;
}

/// Rows matching free text (label) or a pressed combination.
export function filterShortcutCommands(
  commands: readonly ShortcutCommand[],
  query: string,
  labels: (command: ShortcutCommand) => string,
  overrides: Record<string, string[]> | undefined,
  legacy: LegacyShortcutFields,
  platform: ShortcutPlatform,
): ShortcutCommand[] {
  const trimmed = query.trim();
  if (!trimmed) return [...commands];
  const canonical = canonicalBinding(trimmed, platform);
  const needle = trimmed.toLowerCase();
  return commands.filter((command) => {
    if (labels(command).toLowerCase().includes(needle)) return true;
    if (!canonical) return false;
    return effectiveBindings(command.id, overrides, legacy, commands).some(
      (entry) => canonicalBinding(entry, platform) === canonical,
    );
  });
}
