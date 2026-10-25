export type RefreshDataset = "spend" | "spendHistory" | "spendDaily" | "creditExtras" | "creditTrend" | "meterTrend" | "qoder" | "rtk";

interface RefreshRequest {
  usageOnly: boolean;
  interactive: boolean;
}

export class RefreshState {
  private readonly accepted = new Map<string, number>();
  private active: number | null = null;
  private latest = 0;
  private queued: RefreshRequest | null = null;
  private readonly waiters: Array<{ generation: number; resolve: () => void }> = [];
  private readonly cardInFlight = new Set<string>();
  /// Global generation when the card request started. A global refresh that
  /// completes first owns the newer snapshot, so the card result is dropped.
  private readonly cardBase = new Map<string, number>();
  private readonly cardSeq = new Map<string, number>();

  get busy(): boolean {
    return this.active !== null;
  }

  get generation(): number {
    return this.latest;
  }

  cardBusy(id: string): boolean {
    return this.cardInFlight.has(id);
  }

  /// Claim a single-card refresh. False means one is already in flight for
  /// that card — the caller should keep its spinner and wait.
  requestCard(id: string): boolean {
    if (this.cardInFlight.has(id)) return false;
    this.cardInFlight.add(id);
    this.cardBase.set(id, this.latest);
    this.cardSeq.set(id, (this.cardSeq.get(id) ?? 0) + 1);
    return true;
  }

  finishCard(id: string): void {
    this.cardInFlight.delete(id);
    this.cardBase.delete(id);
  }

  /// Adopt a card result only if no full refresh started/finished since the
  /// request. Uses the same monotonic generation rule as `acceptResult`.
  acceptCardResult<T>(id: string, result: T | null): result is T {
    if (result === null || !this.cardInFlight.has(id)) return false;
    if (this.cardBase.get(id) !== this.latest) return false;
    const key = `card:${id}`;
    const generation = this.cardSeq.get(id) ?? 0;
    if (generation < (this.accepted.get(key) ?? 0)) return false;
    this.accepted.set(key, generation);
    return true;
  }

  request(force: boolean, usageOnly: boolean, interactive: boolean): number | null {
    if (this.busy) {
      if (force) {
        this.queued = {
          usageOnly: this.queued ? this.queued.usageOnly && usageOnly : usageOnly,
          interactive: interactive || (this.queued?.interactive ?? false),
        };
      }
      return null;
    }
    this.active = ++this.latest;
    return this.active;
  }

  complete(generation: number): RefreshRequest | null {
    if (generation !== this.active) return null;
    this.active = null;
    const queued = this.queued;
    this.queued = null;
    for (let index = this.waiters.length - 1; index >= 0; index -= 1) {
      if (this.waiters[index].generation <= generation) this.waiters.splice(index, 1)[0].resolve();
    }
    return queued;
  }

  waitForNextAttempt(): Promise<void> {
    const generation = this.latest + 1;
    return new Promise(resolve => this.waiters.push({ generation, resolve }));
  }

  acceptResult<T>(dataset: RefreshDataset, generation: number, result: T | null): result is T {
    if (result === null || generation < (this.accepted.get(dataset) ?? 0)) return false;
    this.accepted.set(dataset, generation);
    return true;
  }}
