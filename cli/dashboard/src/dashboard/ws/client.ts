import WebSocket from "ws";
import type { DashboardFilters, RegistryEvent } from "../types";
import { parseRegistryEvent } from "./protocol";

export type WsClientEvents = {
  open: () => void;
  close: (params: { code: number; reason: string }) => void;
  error: (params: { message: string }) => void;
  event: (ev: RegistryEvent) => void;
  latency: (latencyMs: number) => void;
};

export class RegistryWsClient {
  private ws: WebSocket | undefined;
  private pingTimer: NodeJS.Timeout | undefined;
  private reconnectTimer: NodeJS.Timeout | undefined;
  private listeners: {
    [K in keyof WsClientEvents]?: Array<WsClientEvents[K]>;
  } = {};
  private attempt = 0;
  private lastPingAt: number | undefined;

  constructor(private readonly wsUrl: string) {}

  on<K extends keyof WsClientEvents>(
    event: K,
    listener: WsClientEvents[K],
  ): () => void {
    const arr = (this.listeners[event] ??= []);
    arr.push(listener);
    return () => {
      const next = (this.listeners[event] ?? []).filter((l) => l !== listener);
      this.listeners[event] = next;
    };
  }

  connect(filters: DashboardFilters): void {
    this.clearReconnect();
    this.attempt = 0;
    this.open(filters);
  }

  close(): void {
    this.clearReconnect();
    this.clearPing();
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.close(1000, "client_exit");
    } else {
      this.ws?.terminate();
    }
    this.ws = undefined;
  }

  sendJson(payload: unknown): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    try {
      this.ws.send(JSON.stringify(payload));
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      this.emit("error", { message });
    }
  }

  private open(filters: DashboardFilters): void {
    this.attempt += 1;
    const ws = new WebSocket(this.wsUrl);
    this.ws = ws;

    ws.on("open", () => {
      this.attempt = 0;
      this.emit("open", undefined);
      // Send current filters so servers can do server-side filtering when available.
      this.sendJson({ type: "subscribe", payload: { filters } });
      this.startPing();
    });

    ws.on("message", (data) => {
      const parsed = tryParseJson(data);
      const ev = parseRegistryEvent(parsed);
      if (ev) this.emit("event", ev);
    });

    ws.on("pong", () => {
      if (this.lastPingAt === undefined) return;
      const latencyMs = Date.now() - this.lastPingAt;
      this.emit("latency", latencyMs);
      this.lastPingAt = undefined;
    });

    ws.on("close", (code, reason) => {
      this.clearPing();
      this.emit("close", { code, reason: reason.toString() });
    });

    ws.on("error", (err) => {
      const message = err instanceof Error ? err.message : String(err);
      this.emit("error", { message });
    });
  }

  scheduleReconnect(params: {
    filters: DashboardFilters;
    lastError?: string;
  }): { attempt: number; nextRetryAt: number } {
    this.clearReconnect();
    const attempt = Math.max(1, this.attempt + 1);
    const delay = computeBackoffDelayMs(attempt);
    const nextRetryAt = Date.now() + delay;
    this.reconnectTimer = setTimeout(() => this.open(params.filters), delay);
    return { attempt, nextRetryAt };
  }

  private startPing(): void {
    this.clearPing();
    this.pingTimer = setInterval(() => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
      try {
        this.lastPingAt = Date.now();
        this.ws.ping();
      } catch {
        this.lastPingAt = undefined;
      }
    }, 5_000);
  }

  private clearPing(): void {
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.pingTimer = undefined;
    this.lastPingAt = undefined;
  }

  private clearReconnect(): void {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = undefined;
  }

  private emit<K extends keyof WsClientEvents>(
    event: K,
    payload: Parameters<WsClientEvents[K]>[0] | undefined,
  ): void {
    const list = this.listeners[event] ?? [];
    for (const l of list) {
      (l as any)(payload);
    }
  }
}

function computeBackoffDelayMs(attempt: number): number {
  const base = 500;
  const max = 30_000;
  const exp = Math.min(max, base * 2 ** Math.min(10, attempt));
  const jitter = Math.floor(Math.random() * 250);
  return Math.min(max, exp + jitter);
}

function tryParseJson(data: WebSocket.RawData): unknown {
  try {
    const text = typeof data === "string" ? data : data.toString("utf8");
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}
