export type RefreshDataset = "spend" | "spendHistory" | "spendDaily" | "creditExtras" | "creditTrend" | "meterTrend";

interface RefreshRequest {
  usageOnly: boolean;
  interactive: boolean;
}

export class RefreshState {
  private readonly accepted = new Map<RefreshDataset, number>();
  private active: number | null = null;
  private latest = 0;
  private queued: RefreshRequest | null = null;
  private readonly waiters: Array<{ generation: number; resolve: () => void }> = [];

  get busy(): boolean {
    return this.active !== null;
  }

  get generation(): number {
    return this.latest;
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
  }
}
