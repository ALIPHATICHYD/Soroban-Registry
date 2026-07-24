import { EventEmitter } from "node:events";
import type {
  ActivityBucket,
  ConnectionState,
  DashboardFilters,
  DashboardState,
  Deployment,
  Interaction,
  LoadingState,
} from "../types";

export type StoreChange = {
  state: DashboardState;
  reason: string;
};

const MAX_DEPLOYMENTS = 200;
const MAX_INTERACTIONS = 1000;

export class DashboardStore {
  private readonly emitter = new EventEmitter();
  private state: DashboardState;

  constructor(params: { wsUrl: string; filters: DashboardFilters }) {
    const connection: ConnectionState = {
      status: "disconnected",
      wsUrl: params.wsUrl,
    };
    this.state = {
      connection,
      filters: params.filters,
      deployments: [],
      interactions: [],
      activity: [],
      nowTs: Date.now(),
      loading: {
        deployments: true,
        trending: true,
        activity: true,
      },
    };
  }

  // Central in-memory store; UI subscribes to "change" events.
  getState(): DashboardState {
    return this.state;
  }

  subscribe(listener: (change: StoreChange) => void): () => void {
    this.emitter.on("change", listener);
    return () => this.emitter.off("change", listener);
  }

  setConnection(next: ConnectionState, reason: string): void {
    this.state = { ...this.state, connection: next };
    this.emit(reason);
  }

  setFilters(next: DashboardFilters, reason: string): void {
    this.state = { ...this.state, filters: next };
    this.emit(reason);
  }

  tickNow(nowTs: number): void {
    this.state = { ...this.state, nowTs };
    this.emit("tick");
  }

  setLoading(loading: Partial<LoadingState>, reason: string): void {
    this.state = {
      ...this.state,
      loading: { ...this.state.loading, ...loading },
    };
    this.emit(reason);
  }

  addDeployment(dep: Deployment): void {
    const deployments = [dep, ...this.state.deployments].slice(
      0,
      MAX_DEPLOYMENTS,
    );
    this.state = {
      ...this.state,
      deployments,
      loading: { ...this.state.loading, deployments: false },
    };
    this.bumpActivity(dep.ts, { deployments: 1, interactions: 0 });
    this.emit("deployment_created");
  }

  addInteraction(intx: Interaction): void {
    const interactions = [intx, ...this.state.interactions].slice(
      0,
      MAX_INTERACTIONS,
    );
    this.state = {
      ...this.state,
      interactions,
      loading: { ...this.state.loading, trending: false },
    };
    this.bumpActivity(intx.ts, { deployments: 0, interactions: 1 });
    this.emit("contract_interaction");
  }

  private bumpActivity(
    ts: number,
    delta: { deployments: number; interactions: number },
  ): void {
    const bucketMs = 60_000;
    const startTs = Math.floor(ts / bucketMs) * bucketMs;

    const activity = this.state.activity.slice();
    const last = activity[activity.length - 1];
    if (!last || last.startTs !== startTs) {
      activity.push({ startTs, deployments: 0, interactions: 0 });
    }

    const idx = activity.length - 1;
    const updated: ActivityBucket = {
      ...activity[idx],
      deployments: activity[idx].deployments + delta.deployments,
      interactions: activity[idx].interactions + delta.interactions,
    };

    activity[idx] = updated;

    const maxBuckets = 120;
    const trimmed = activity.slice(Math.max(0, activity.length - maxBuckets));
    this.state = {
      ...this.state,
      activity: trimmed,
      loading: { ...this.state.loading, activity: false },
    };
  }

  private emit(reason: string): void {
    this.emitter.emit("change", {
      state: this.state,
      reason,
    } satisfies StoreChange);
  }
}
