import type { DashboardFilters } from "./types";
import { DashboardStore } from "./state/store";
import { RenderScheduler } from "./render/render_scheduler";
import { RegistryWsClient } from "./ws/client";
import { DashboardApp } from "./ui/dashboard_app";
import { advanceAnimationFrame } from "./ui/skeleton";

export async function runDashboard(params: {
  refreshRateMs: number;
  network?: string;
  category?: string;
}): Promise<void> {
  const wsUrl = process.env.SOROBAN_REGISTRY_WS_URL ?? "ws://127.0.0.1:8787";

  const initialFilters: DashboardFilters = {
    network: params.network,
    category: params.category,
  };

  const store = new DashboardStore({ wsUrl, filters: initialFilters });
  const ws = new RegistryWsClient(wsUrl);

  let shuttingDown = false;
  let tickTimer: NodeJS.Timeout | undefined;
  let resolveDone: (() => void) | undefined;
  const done = new Promise<void>((resolve) => {
    resolveDone = resolve;
  });

  const app = new DashboardApp({
    getState: () => store.getState(),
    onQuit: () => {
      shuttingDown = true;
      resolveDone?.();
    },
    onRefresh: () => {
      ws.sendJson({ type: "refresh", payload: {} });
      scheduler.request();
    },
    onSetFilters: (filters) => {
      store.setFilters(filters, "filters_changed");
      ws.sendJson({ type: "set_filters", payload: { filters } });
      scheduler.request();
    },
    requestRender: () => scheduler.request(),
  });

  const scheduler = new RenderScheduler(params.refreshRateMs, () => {
    app.renderFromState(store.getState());
    app.render();
  });

  const cleanup = () => {
    shuttingDown = true;
    if (tickTimer) clearInterval(tickTimer);
    ws.close();
    app.destroy();
  };

  store.subscribe(() => scheduler.request());

  ws.on("open", () => {
    store.setConnection(
      { status: "connected", wsUrl, connectedAt: Date.now() },
      "ws_open",
    );
  });

  ws.on("latency", (latencyMs) => {
    const current = store.getState().connection;
    if (current.status === "connected") {
      store.setConnection({ ...current, latencyMs }, "latency");
    }
  });

  ws.on("event", (ev) => {
    if (ev.type === "deployment_created") {
      store.addDeployment({
        id: ev.payload.id,
        contractId: ev.payload.contractId,
        network: ev.payload.network,
        category: ev.payload.category,
        publisher: ev.payload.publisher,
        ts: Date.parse(ev.payload.timestamp) || Date.now(),
      });
      return;
    }

    if (ev.type === "contract_interaction") {
      store.addInteraction({
        id: ev.payload.id,
        contractId: ev.payload.contractId,
        network: ev.payload.network,
        ts: Date.parse(ev.payload.timestamp) || Date.now(),
      });
      return;
    }

    if (ev.type === "network_status") {
      const conn = store.getState().connection;
      if (conn.status === "connected") {
        store.setConnection(
          { ...conn, latencyMs: ev.payload.latencyMs },
          "network_status",
        );
      }
    }
  });

  ws.on("error", ({ message }) => {
    const conn = store.getState().connection;
    if (conn.status === "connected") {
      store.setConnection(
        {
          status: "disconnected",
          wsUrl,
          lastConnectedAt: conn.connectedAt,
          lastError: message,
        },
        "ws_error",
      );
    } else {
      store.setConnection(
        { ...conn, status: "disconnected", lastError: message },
        "ws_error",
      );
    }
  });

  ws.on("close", ({ code, reason }) => {
    if (shuttingDown) return;
    const lastError = reason || `closed (${code})`;
    const conn = store.getState().connection;
    const lastConnectedAt =
      conn.status === "connected" ? conn.connectedAt : conn.lastConnectedAt;
    const { attempt, nextRetryAt } = ws.scheduleReconnect({
      filters: store.getState().filters,
      lastError,
    });
    store.setConnection(
      {
        status: "reconnecting",
        wsUrl,
        attempt,
        nextRetryAt,
        lastError,
        lastConnectedAt,
      },
      "ws_close",
    );
  });

  ws.connect(store.getState().filters);

  tickTimer = setInterval(() => {
    store.tickNow(Date.now());
    advanceAnimationFrame();
    scheduler.request();
  }, 1_000);
  scheduler.request();

  await done;
  cleanup();
}
