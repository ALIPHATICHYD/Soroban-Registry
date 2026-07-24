import blessed from "blessed";
import type { DashboardFilters, DashboardState } from "../types";
import {
  selectFilteredDeployments,
  selectTrendingContracts,
} from "../state/selectors";
import { sparkline } from "../render/sparkline";
import { clampStr, formatSince } from "../util/format";
import { parseKeyValueFilters, promptLine } from "./modals";
import {
  createSkeletonList,
  createSkeleton,
  createAccessibleLoadingIndicator,
} from "./skeleton";

type FocusedPanel = "deployments" | "trending";

export class DashboardApp {
  private readonly screen: blessed.Widgets.Screen;
  private readonly header: blessed.Widgets.BoxElement;
  private readonly deploymentsList: blessed.Widgets.ListElement;
  private readonly trendingList: blessed.Widgets.ListElement;
  private readonly activityBox: blessed.Widgets.BoxElement;

  private focused: FocusedPanel = "deployments";
  private modalActive = false;

  private lastHeader = "";
  private lastDeploymentsKey = "";
  private lastTrendingKey = "";
  private lastActivityKey = "";

  constructor(
    private readonly params: {
      getState: () => DashboardState;
      onQuit: () => void;
      onRefresh: () => void;
      onSetFilters: (filters: DashboardFilters) => void;
      requestRender: () => void;
    },
  ) {
    this.screen = blessed.screen({
      smartCSR: true,
      fullUnicode: true,
      title: "Soroban Registry Dashboard",
      dockBorders: true,
    });

    this.screen.key(["C-c", "q"], () => this.params.onQuit());
    this.screen.key(["r"], () => this.params.onRefresh());
    this.screen.key(["left", "right", "tab"], () => this.toggleFocus());
    this.screen.key(["up"], () => this.onArrow("up"));
    this.screen.key(["down"], () => this.onArrow("down"));
    this.screen.key(["f"], () => void this.openFilterModal());
    this.screen.key(["/"], () => void this.openSearchModal());

    this.header = blessed.box({
      parent: this.screen,
      top: 0,
      left: 0,
      right: 0,
      height: 3,
      border: "line",
      style: { border: { fg: "cyan" } },
    });

    const midTop = 3;
    const bottomHeight = 9;
    const midHeight = `100%-${midTop + bottomHeight}`;

    this.deploymentsList = blessed.list({
      parent: this.screen,
      label: " Recent deployments ",
      top: midTop,
      left: 0,
      width: "50%",
      height: midHeight,
      border: "line",
      keys: false,
      mouse: false,
      tags: false,
      style: {
        border: { fg: "gray" },
        selected: { bg: "blue" },
      },
      scrollbar: {
        ch: " ",
        track: { bg: "gray" },
        style: { bg: "yellow" },
      },
    });

    this.trendingList = blessed.list({
      parent: this.screen,
      label: " Trending contracts ",
      top: midTop,
      left: "50%",
      width: "50%",
      height: midHeight,
      border: "line",
      keys: false,
      mouse: false,
      tags: false,
      style: {
        border: { fg: "gray" },
        selected: { bg: "blue" },
      },
      scrollbar: {
        ch: " ",
        track: { bg: "gray" },
        style: { bg: "yellow" },
      },
    });

    this.activityBox = blessed.box({
      parent: this.screen,
      label: " Activity (last 120 minutes) ",
      left: 0,
      right: 0,
      bottom: 0,
      height: bottomHeight,
      border: "line",
      style: { border: { fg: "gray" } },
    });

    this.applyFocusStyles();
    this.renderFromState(this.params.getState());
    this.render();

    this.screen.on("resize", () => this.params.requestRender());
  }

  destroy(): void {
    this.screen.destroy();
  }

  render(): void {
    this.screen.render();
  }

  renderFromState(state: DashboardState): void {
    this.renderHeader(state);
    this.renderDeployments(state);
    this.renderTrending(state);
    this.renderActivity(state);
  }

  private renderHeader(state: DashboardState): void {
    const conn = state.connection;
    const filters = state.filters;

    let status = "DISCONNECTED";
    let statusColor: "red" | "green" | "yellow" = "red";
    let details = "";

    if (conn.status === "connected") {
      status = "CONNECTED";
      statusColor = "green";
      details =
        conn.latencyMs !== undefined
          ? `latency ${conn.latencyMs}ms`
          : "latency --";
    } else if (conn.status === "reconnecting") {
      status = "RECONNECTING";
      statusColor = "yellow";
      const inSec = Math.max(
        0,
        Math.ceil((conn.nextRetryAt - Date.now()) / 1000),
      );
      details = `retry in ${inSec}s (attempt ${conn.attempt})`;
    } else {
      details = conn.lastError
        ? `error: ${conn.lastError}`
        : "waiting for connection";
    }

    const f = `network=${filters.network ?? "*"} category=${filters.category ?? "*"} query=${filters.query ?? ""}`;

    const content = [
      ` {bold}Network:{/bold} ${filters.network ?? "all"}   {bold}WS:{/bold} ${conn.wsUrl}`,
      ` {bold}Status:{/bold} {${statusColor}-fg}${status}{/${statusColor}-fg}   ${details}`,
      ` {bold}Keys:{/bold} q quit · r refresh · f filters · / search · ←/→ switch panel · ↑/↓ scroll   ${f}`,
    ].join("\n");

    if (content !== this.lastHeader) {
      this.lastHeader = content;
      this.header.setContent(content);
      this.header.setTags(true);
    }
    this.header.style.border = { fg: statusColor };
  }

  private renderDeployments(state: DashboardState): void {
    const width = Math.max(
      10,
      Math.floor((this.deploymentsList.width as number) ?? 40) - 4,
    );

    // Show skeleton while loading
    if (state.loading.deployments) {
      const loadingIndicator = createAccessibleLoadingIndicator(
        "deployments",
        width,
      );
      const skeletonItems = createSkeletonList(10, width, "table-row");
      this.deploymentsList.setItems([loadingIndicator, ...skeletonItems]);
      return;
    }

    const items = selectFilteredDeployments(state).slice(0, 100);
    const key = `${items.length}:${items[0]?.id ?? ""}:${state.nowTs}`;
    if (key === this.lastDeploymentsKey) return;
    this.lastDeploymentsKey = key;

    const lines = items.map((d) => {
      const age = formatSince(d.ts, state.nowTs).padStart(3, " ");
      const net = clampStr(d.network, 10).padEnd(10, " ");
      const cat = clampStr(d.category ?? "-", 8).padEnd(8, " ");
      const id = clampStr(d.contractId, 40);
      const line = `${age}  ${net}  ${cat}  ${id}`;
      return clampStr(line, width);
    });

    this.deploymentsList.setItems(lines.length ? lines : ["(no deployments)"]);
  }

  private renderTrending(state: DashboardState): void {
    const width = Math.max(
      10,
      Math.floor((this.trendingList.width as number) ?? 40) - 4,
    );

    // Show skeleton while loading
    if (state.loading.trending) {
      const loadingIndicator = createAccessibleLoadingIndicator(
        "trending contracts",
        width,
      );
      const skeletonItems = createSkeletonList(10, width, "table-row");
      this.trendingList.setItems([loadingIndicator, ...skeletonItems]);
      return;
    }

    const items = selectTrendingContracts(state, {
      windowMs: 10 * 60_000,
      limit: 100,
    });

    const key = `${items.length}:${items[0]?.contractId ?? ""}:${state.nowTs}`;
    if (key === this.lastTrendingKey) return;
    this.lastTrendingKey = key;

    const lines = items.map((t) => {
      const age = formatSince(t.lastTs, state.nowTs).padStart(3, " ");
      const net = clampStr(t.network, 10).padEnd(10, " ");
      const count = String(t.count).padStart(5, " ");
      const id = clampStr(t.contractId, 40);
      const line = `${age}  ${net}  ${count}  ${id}`;
      return clampStr(line, width);
    });

    this.trendingList.setItems(lines.length ? lines : ["(no interactions)"]);
  }

  private renderActivity(state: DashboardState): void {
    const width = Math.max(20, (this.activityBox.width as number) - 4);

    // Show skeleton while loading
    if (state.loading.activity) {
      const loadingIndicator = createAccessibleLoadingIndicator(
        "activity data",
        width,
      );
      const skeletonContent = [
        loadingIndicator,
        createSkeleton({ width, height: 1, variant: "line" }),
        createSkeleton({ width, height: 1, variant: "line" }),
        "",
        createSkeleton({ width, height: 1, variant: "line" }),
      ].join("\n");
      this.activityBox.setContent(skeletonContent);
      return;
    }

    const buckets = state.activity.slice(-120);
    const deployments = buckets.map((b) => b.deployments);
    const interactions = buckets.map((b) => b.interactions);

    const key = `${width}:${buckets.length}:${buckets[buckets.length - 1]?.startTs ?? 0}`;
    if (key === this.lastActivityKey) return;
    this.lastActivityKey = key;

    const dLine = sparkline(deployments, { width });
    const iLine = sparkline(interactions, { width });

    const dMax = deployments.length ? Math.max(...deployments) : 0;
    const iMax = interactions.length ? Math.max(...interactions) : 0;

    this.activityBox.setContent(
      [
        `Deployments: ${dLine}  max ${dMax}`,
        `Interactions: ${iLine}  max ${iMax}`,
        "",
        "Tip: use f to filter (network/category), / to search by contract id.",
      ].join("\n"),
    );
  }

  private toggleFocus(): void {
    if (this.modalActive) return;
    this.focused = this.focused === "deployments" ? "trending" : "deployments";
    this.applyFocusStyles();
    this.params.requestRender();
  }

  private applyFocusStyles(): void {
    const depFocused = this.focused === "deployments";
    this.deploymentsList.style.border = { fg: depFocused ? "yellow" : "gray" };
    this.trendingList.style.border = { fg: depFocused ? "gray" : "yellow" };
  }

  private onArrow(dir: "up" | "down"): void {
    if (this.modalActive) return;
    const list =
      this.focused === "deployments" ? this.deploymentsList : this.trendingList;
    if (dir === "up") list.up(1);
    else list.down(1);
    this.params.requestRender();
  }

  private async openFilterModal(): Promise<void> {
    if (this.modalActive) return;
    this.modalActive = true;
    try {
      const current = this.params.getState();
      const line = await promptLine({
        screen: this.screen,
        title: "Filters",
        hint: "Enter: network=<name> category=<type>. Use empty value to clear, e.g. network= category=.\nExample: network=testnet category=dex",
        initial: `network=${current.filters.network ?? ""} category=${current.filters.category ?? ""}`,
      });
      if (line === undefined) return;

      const parsed = parseKeyValueFilters(line);
      const next: DashboardFilters = {
        ...current.filters,
        ...parsed,
        query: current.filters.query,
      };
      this.params.onSetFilters(next);
    } finally {
      this.modalActive = false;
      this.params.requestRender();
    }
  }

  private async openSearchModal(): Promise<void> {
    if (this.modalActive) return;
    this.modalActive = true;
    try {
      const current = this.params.getState();
      const line = await promptLine({
        screen: this.screen,
        title: "Search",
        hint: "Enter a substring to match contract id/publisher. Empty clears search.",
        initial: current.filters.query ?? "",
      });
      if (line === undefined) return;
      const next: DashboardFilters = {
        ...current.filters,
        query: line.trim() ? line.trim() : undefined,
      };
      this.params.onSetFilters(next);
    } finally {
      this.modalActive = false;
      this.params.requestRender();
    }
  }
}
