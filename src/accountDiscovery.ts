// Local account discovery baseline (#63-09). First sight only records a
// folded fingerprint; a changed fingerprint means a new sign-in on the same
// local slot, and a vanished identity drops its baseline. Raw identities are
// never stored — the key names the slot, the value is an FNV-1a fold.

export type DiscoveryBaseline = Record<string, string>;

export interface DiscoveryResult {
  baseline: DiscoveryBaseline;
  discovered: string[];
  removed: string[];
}

export function fingerprintIdentity(parts: ReadonlyArray<string | null | undefined>): string {
  const raw = parts.map((part) => part ?? "").join("\u0001");
  let hash = 0x811c9dc5;
  for (let index = 0; index < raw.length; index += 1) {
    hash ^= raw.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

export function reconcileDiscovery(
  previous: DiscoveryBaseline,
  current: DiscoveryBaseline,
): DiscoveryResult {
  const baseline: DiscoveryBaseline = { ...previous };
  const discovered: string[] = [];
  const removed: string[] = [];
  for (const [key, fingerprint] of Object.entries(current)) {
    if (!(key in previous)) {
      // First sight is a silent baseline, never a notification.
      baseline[key] = fingerprint;
    } else if (previous[key] !== fingerprint) {
      baseline[key] = fingerprint;
      discovered.push(key);
    }
  }
  for (const key of Object.keys(previous)) {
    if (!(key in current)) {
      delete baseline[key];
      removed.push(key);
    }
  }
  return { baseline, discovered, removed };
}
