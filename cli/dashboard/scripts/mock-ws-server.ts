import { WebSocketServer } from "ws";
import { randomUUID } from "node:crypto";

type ClientFilters = {
  network?: string;
  category?: string;
  query?: string;
};

type ClientState = {
  filters: ClientFilters;
};

const PORT = Number.parseInt(process.env.MOCK_WS_PORT ?? "8787", 10);
const wss = new WebSocketServer({ port: PORT });

const clients = new Map<any, ClientState>();

const NETWORKS = ["mainnet", "testnet", "futurenet"] as const;
const CATEGORIES = ["dex", "nft", "token", "oracle", "lending"] as const;

function nowIso(): string {
  return new Date().toISOString();
}

function randomFrom<T>(arr: readonly T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

function makeContractId(): string {
  const suffix = Math.random().toString(16).slice(2, 10);
  return `C${suffix}`.toUpperCase();
}

function passesFilters(
  ev: { network: string; category?: string; contractId: string },
  filters: ClientFilters,
): boolean {
  if (filters.network && ev.network !== filters.network) return false;
  if (filters.category && ev.category !== filters.category) return false;
  if (filters.query) {
    const q = filters.query.toLowerCase();
    if (!ev.contractId.toLowerCase().includes(q)) return false;
  }
  return true;
}

wss.on("connection", (ws) => {
  clients.set(ws, { filters: {} });

  ws.on("message", (data) => {
    const msg = safeJson(data.toString());
    if (!msg || typeof msg.type !== "string") return;

    if (
      msg.type === "subscribe" &&
      msg.payload &&
      typeof msg.payload === "object"
    ) {
      const next = (msg.payload as any).filters;
      if (next && typeof next === "object") {
        clients.set(ws, { filters: sanitizeFilters(next) });
      }
      return;
    }

    if (
      msg.type === "set_filters" &&
      msg.payload &&
      typeof msg.payload === "object"
    ) {
      const next = (msg.payload as any).filters;
      if (next && typeof next === "object") {
        clients.set(ws, { filters: sanitizeFilters(next) });
      }
      return;
    }

    if (msg.type === "refresh") {
      for (let i = 0; i < 5; i++) emitInteraction(ws);
      emitDeployment(ws);
      return;
    }
  });

  ws.on("close", () => {
    clients.delete(ws);
  });
});

setInterval(() => {
  for (const ws of wss.clients) {
    if (ws.readyState !== ws.OPEN) continue;
    emitNetworkStatus(ws);
  }
}, 3_000);

setInterval(() => {
  for (const ws of wss.clients) {
    if (ws.readyState !== ws.OPEN) continue;
    emitInteraction(ws);
    if (Math.random() < 0.15) emitDeployment(ws);
  }
}, 1_000);

process.stdout.write(`Mock WS server running at ws://127.0.0.1:${PORT}\n`);

function emitDeployment(ws: any): void {
  const filters = clients.get(ws)?.filters ?? {};
  const network = randomFrom(NETWORKS);
  const category = randomFrom(CATEGORIES);
  const contractId = makeContractId();

  const ev = { network, category, contractId };
  if (!passesFilters(ev, filters)) return;

  ws.send(
    JSON.stringify({
      type: "deployment_created",
      payload: {
        id: randomUUID(),
        contractId,
        network,
        category,
        publisher: `G${Math.random().toString(36).slice(2, 10).toUpperCase()}`,
        timestamp: nowIso(),
      },
    }),
  );
}

function emitInteraction(ws: any): void {
  const filters = clients.get(ws)?.filters ?? {};
  const network = randomFrom(NETWORKS);
  const contractId = makeContractId();

  const ev = { network, contractId };
  if (!passesFilters(ev, filters)) return;

  ws.send(
    JSON.stringify({
      type: "contract_interaction",
      payload: {
        id: randomUUID(),
        contractId,
        network,
        timestamp: nowIso(),
      },
    }),
  );
}

function emitNetworkStatus(ws: any): void {
  const filters = clients.get(ws)?.filters ?? {};
  const network = filters.network ?? randomFrom(NETWORKS);
  const latencyMs = Math.floor(20 + Math.random() * 180);

  ws.send(
    JSON.stringify({
      type: "network_status",
      payload: {
        network,
        status: "connected",
        latencyMs,
        timestamp: nowIso(),
      },
    }),
  );
}

function safeJson(text: string): any | undefined {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

function sanitizeFilters(input: any): ClientFilters {
  const out: ClientFilters = {};
  if (typeof input.network === "string")
    out.network = input.network || undefined;
  if (typeof input.category === "string")
    out.category = input.category || undefined;
  if (typeof input.query === "string") out.query = input.query || undefined;
  return out;
}
