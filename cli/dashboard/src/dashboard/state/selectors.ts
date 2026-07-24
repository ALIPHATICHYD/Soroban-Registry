import type { DashboardState, Deployment } from "../types";

export function selectFilteredDeployments(state: DashboardState): Deployment[] {
  const { network, category, query } = state.filters;
  const q = query?.trim().toLowerCase();

  return state.deployments.filter((d) => {
    if (network && d.network !== network) return false;
    if (category && d.category !== category) return false;
    if (
      q &&
      !d.contractId.toLowerCase().includes(q) &&
      !(d.publisher ?? "").toLowerCase().includes(q)
    )
      return false;
    return true;
  });
}

export function selectTrendingContracts(
  state: DashboardState,
  params: { windowMs: number; limit: number },
): Array<{
  contractId: string;
  network: string;
  count: number;
  lastTs: number;
}> {
  const cutoff = state.nowTs - params.windowMs;
  const { network, query } = state.filters;
  const q = query?.trim().toLowerCase();

  const counts = new Map<
    string,
    { contractId: string; network: string; count: number; lastTs: number }
  >();

  for (const i of state.interactions) {
    if (i.ts < cutoff) continue;
    if (network && i.network !== network) continue;

    const key = `${i.network}:${i.contractId}`;
    const prev = counts.get(key);
    const next = prev
      ? { ...prev, count: prev.count + 1, lastTs: Math.max(prev.lastTs, i.ts) }
      : {
          contractId: i.contractId,
          network: i.network,
          count: 1,
          lastTs: i.ts,
        };
    counts.set(key, next);
  }

  const items = Array.from(counts.values()).sort(
    (a, b) => b.count - a.count || b.lastTs - a.lastTs,
  );
  const filtered = q
    ? items.filter((x) => x.contractId.toLowerCase().includes(q))
    : items;
  return filtered.slice(0, params.limit);
}
