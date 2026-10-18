export class RenderScheduler {
  private dirty = false;
  private frame: number | null = null;
  private readonly render: () => void;
  private readonly visible: () => boolean;
  private readonly requestFrame: (callback: () => void) => number;
  private readonly cancelFrame: (frame: number) => void;

  constructor(
    render: () => void,
    visible: () => boolean,
    requestFrame: (callback: () => void) => number,
    cancelFrame: (frame: number) => void,
  ) {
    this.render = render;
    this.visible = visible;
    this.requestFrame = requestFrame;
    this.cancelFrame = cancelFrame;
  }

  get pending(): boolean {
    return this.dirty;
  }

  request(): void {
    this.dirty = true;
    if (!this.visible() || this.frame !== null) return;
    this.frame = this.requestFrame(() => {
      this.frame = null;
      this.flush();
    });
  }

  flush(): void {
    if (!this.dirty || !this.visible()) return;
    if (this.frame !== null) {
      this.cancelFrame(this.frame);
      this.frame = null;
    }
    this.dirty = false;
    this.render();
  }
}
