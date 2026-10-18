/** Rust char::is_whitespace and to_ascii_uppercase, followed by Crockford folding. */
export function normalizeRecoveryCode(input: string): string {
  return input
    .replace(/[-\u0009-\u000D\u0020\u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]/g, "")
    .replace(/[a-z]/g, character => character.toUpperCase())
    .replace(/O/g, "0")
    .replace(/[IL]/g, "1");
}

export function extractRecoveryCode(text: string): string | null {
  for (const part of [text, ...text.split(/\r?\n/)]) {
    const folded = normalizeRecoveryCode(part);
    if (/^[0-9A-HJKMNP-TV-Z]{34}$/.test(folded)) return folded;
  }
  return null;
}
