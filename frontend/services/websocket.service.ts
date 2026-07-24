type MessageHandler = (data: unknown) => void;
type ConnectionHandler = () => void;
type ErrorHandler = (error: unknown) => void;

interface WebSocketMessage {
  type: string;
  data: unknown;
}

export class WebSocketService {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectDelay = 1000;
  private messageHandlers: Map<string, Set<MessageHandler>> = new Map();
  private openHandlers: Set<ConnectionHandler> = new Set();
  private errorHandlers: Set<ErrorHandler> = new Set();
  private pingInterval: ReturnType<typeof setInterval> | null = null;

  constructor(url?: string) {
    this.url = url || this.getWebSocketUrl();
  }

  private getWebSocketUrl(): string {
    const baseUrl = process.env.NEXT_PUBLIC_API_URL || "";
    return baseUrl.replace(/^http/, "ws").replace(/\/$/, "");
  }

  connect(): Promise<void> {
    // Guard: WebSocket is not available in SSR / Node.js environments
    if (typeof window === "undefined" || typeof WebSocket === "undefined") {
      return Promise.resolve();
    }

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(`${this.url}/ws/contracts`);

        this.ws.onopen = () => {
          this.reconnectAttempts = 0;
          this.startPing();
          this.openHandlers.forEach((handler) => handler());
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data) as WebSocketMessage;
            const handlers = this.messageHandlers.get(message.type);
            handlers?.forEach((handler) => handler(message.data));
          } catch (err) {
            console.error("Failed to parse WebSocket message:", err);
          }
        };

        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        this.ws.onerror = (_event) => {
          const error = new Error("WebSocket connection failed");
          this.errorHandlers.forEach((handler) => handler(error));
          // Warn instead of hard-rejecting so callers that don't await
          // connect() don't generate uncaught promise rejections.
          console.warn("[WebSocketService]", error.message);
          reject(error);
        };

        this.ws.onclose = () => {
          this.stopPing();
          this.reconnect();
        };
      } catch (error) {
        reject(error);
      }
    });
  }

  private reconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(
        this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
        30000,
      );
      setTimeout(() => {
        this.connect().catch(console.error);
      }, delay);
    }
  }

  private startPing(): void {
    this.pingInterval = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.send({ type: "ping", data: {} });
      }
    }, 30000); // Ping every 30 seconds
  }

  private stopPing(): void {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  send(message: WebSocketMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  on(type: string, handler: MessageHandler): () => void {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, new Set());
    }
    this.messageHandlers.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.messageHandlers.get(type)?.delete(handler);
    };
  }

  onOpen(handler: ConnectionHandler): () => void {
    this.openHandlers.add(handler);
    return () => this.openHandlers.delete(handler);
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler);
    return () => this.errorHandlers.delete(handler);
  }

  disconnect(): void {
    this.stopPing();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.messageHandlers.clear();
    this.openHandlers.clear();
    this.errorHandlers.clear();
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

// Only instantiate on the client side to avoid SSR errors
export const wsService =
  typeof window !== "undefined" ? new WebSocketService() : (null as unknown as WebSocketService);
