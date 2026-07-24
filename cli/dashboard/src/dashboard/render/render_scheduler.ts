export class RenderScheduler {
  private timer: NodeJS.Timeout | undefined;
  private pending = false;
  private lastRenderAt = 0;

  constructor(
    private readonly minIntervalMs: number,
    private readonly render: () => void,
  ) {}

  // Throttles terminal re-renders so high-frequency WS events don't cause lag.
  request(): void {
    this.pending = true;
    if (this.timer) return;

    const now = Date.now();
    const dueIn = Math.max(0, this.minIntervalMs - (now - this.lastRenderAt));
    this.timer = setTimeout(() => this.flush(), dueIn);
  }

  flush(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;

    if (!this.pending) return;
    this.pending = false;

    this.lastRenderAt = Date.now();
    this.render();
  }
}
