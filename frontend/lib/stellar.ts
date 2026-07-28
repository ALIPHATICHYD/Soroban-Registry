/**
 * Stellar live data adapter
 *
 * Fetches real-time Soroban contract and activity data from:
 *   1. Stellar Horizon REST API  (primary — most reliable, no auth needed)
 *   2. Stellar Expert public API (secondary fallback for contract list)
 *
 * Exports:
 *  - fetchRecentContracts(network, limit) → Contract[]
 *  - fetchContractActivity(network, limit) → AnalyticsEvent[]
 */

import type { Contract, AnalyticsEvent } from "@/types";

export type StellarNetwork = "public" | "testnet";

// ─── Network config ───────────────────────────────────────────────────────────

interface NetworkConfig {
  horizon: string;
  expert: string;
}

const NETWORKS: Record<string, NetworkConfig> = {
  mainnet: {
    horizon: "https://horizon.stellar.org",
    expert: "https://api.stellar.expert/explorer/public",
  },
  testnet: {
    horizon: "https://horizon-testnet.stellar.org",
    expert: "https://api.stellar.expert/explorer/testnet",
  },
  futurenet: {
    horizon: "https://horizon-futurenet.stellar.org",
    expert: "https://api.stellar.expert/explorer/testnet",
  },
};

function getNet(network: string): NetworkConfig {
  return NETWORKS[network] ?? NETWORKS.mainnet;
}

// ─── Horizon types ────────────────────────────────────────────────────────────

interface HorizonOperationRecord {
  id: string;
  type: string; // "invoke_host_function" | "upload_contract_wasm" | etc.
  type_i: number;
  paging_token: string;
  transaction_hash: string;
  source_account: string;
  created_at: string;
  // invoke_host_function fields
  function?: string;
  contract_id?: string;
  // Additional metadata
  transaction_successful?: boolean;
}

interface HorizonOperationsResponse {
  _embedded: {
    records: HorizonOperationRecord[];
  };
  _links: {
    next?: { href: string };
    prev?: { href: string };
  };
}

// ─── Stellar Expert types ─────────────────────────────────────────────────────

interface ExpertContract {
  id: string;
  creator?: string;
  created?: number; // Unix epoch seconds
  wasm?: string;
  payments?: number;
  trades?: number;
}

interface ExpertContractListResponse {
  _embedded: {
    records: ExpertContract[];
  };
}

// ─── Fetch helpers ────────────────────────────────────────────────────────────

async function horizonGet<T>(url: string): Promise<T> {
  const res = await fetch(url, {
    headers: { Accept: "application/json" },
    signal: AbortSignal.timeout(12_000),
  });
  if (!res.ok) throw new Error(`Horizon ${url}: HTTP ${res.status}`);
  return res.json() as Promise<T>;
}

async function expertGet<T>(url: string): Promise<T> {
  const res = await fetch(url, {
    headers: { Accept: "application/json" },
    signal: AbortSignal.timeout(12_000),
  });
  if (!res.ok) throw new Error(`Stellar Expert ${url}: HTTP ${res.status}`);
  return res.json() as Promise<T>;
}

// ─── Map Horizon operation types to our event types ───────────────────────────

function opTypeToEventType(
  typeStr: string,
  fn?: string,
): AnalyticsEvent["event_type"] {
  if (typeStr === "upload_contract_wasm") return "contract_published";
  if (typeStr === "invoke_host_function") {
    if (!fn) return "contract_deployed";
    const f = fn.toLowerCase();
    if (f.includes("upgrade") || f.includes("update")) return "version_created";
    if (f.includes("verify")) return "contract_verified";
    if (f.includes("create") || f.includes("init")) return "contract_deployed";
    return "contract_deployed";
  }
  if (typeStr === "create_account") return "contract_deployed";
  return "contract_updated";
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Fetch most-recently created Soroban contracts.
 *
 * Strategy:
 *   1. Stellar Expert /contract endpoint (has rich Soroban-specific metadata)
 *   2. Fall back to Horizon invoke_host_function ops if Expert fails
 */
export async function fetchRecentContracts(
  network: string = "mainnet",
  limit = 20,
): Promise<Contract[]> {
  const cfg = getNet(network);
  const internalNetwork = (
    network === "testnet" ? "testnet" : "mainnet"
  ) as Contract["network"];

  // ── Attempt 1: Stellar Expert ──────────────────────────────────────────────
  try {
    const url = `${cfg.expert}/contract?limit=${limit}&order=desc`;
    const data = await expertGet<ExpertContractListResponse>(url);
    const records = data?._embedded?.records;
    if (Array.isArray(records) && records.length > 0) {
      return records.map(
        (r): Contract => ({
          id: r.id,
          contract_id: r.id,
          name: shortenId(r.id),
          description: undefined,
          network: internalNetwork,
          publisher_id: r.creator ?? "unknown",
          category: undefined,
          tags: [],
          is_verified: false,
          wasm_hash: r.wasm ?? "",
          created_at: r.created
            ? new Date(r.created * 1000).toISOString()
            : new Date().toISOString(),
          updated_at: r.created
            ? new Date(r.created * 1000).toISOString()
            : new Date().toISOString(),
          popularity_score: (r.payments ?? 0) + (r.trades ?? 0),
          downloads: undefined,
          average_rating: undefined,
          avg_rating: undefined,
          review_count: 0,
        }),
      );
    }
  } catch {
    // Expert failed → try Horizon
  }

  // ── Attempt 2: Horizon invoke_host_function ops ───────────────────────────
  const url =
    `${cfg.horizon}/operations?join=transactions` +
    `&type=invoke_host_function&limit=${limit}&order=desc`;
  const data = await horizonGet<HorizonOperationsResponse>(url);
  const records = data?._embedded?.records ?? [];

  // Deduplicate by contract_id so each contract appears once
  const seen = new Set<string>();
  const contracts: Contract[] = [];

  for (const r of records) {
    const cid = r.contract_id ?? r.transaction_hash;
    if (seen.has(cid)) continue;
    seen.add(cid);
    contracts.push({
      id: cid,
      contract_id: cid,
      name: r.contract_id ? shortenId(r.contract_id) : shortenId(r.transaction_hash),
      description: undefined,
      network: internalNetwork,
      publisher_id: r.source_account,
      category: undefined,
      tags: [],
      is_verified: false,
      wasm_hash: "",
      created_at: r.created_at,
      updated_at: r.created_at,
      popularity_score: 0,
      downloads: undefined,
      average_rating: undefined,
      avg_rating: undefined,
      review_count: 0,
    });

    if (contracts.length >= limit) break;
  }

  return contracts;
}

/**
 * Fetch recent Soroban-related on-chain operations shaped as AnalyticsEvents.
 *
 * Uses Horizon operations endpoint filtered to Soroban operation types.
 * Falls back to Stellar Expert operations if Horizon fails.
 */
export async function fetchContractActivity(
  network: string = "mainnet",
  limit = 30,
): Promise<AnalyticsEvent[]> {
  const cfg = getNet(network);
  const internalNetwork = (
    network === "testnet" ? "testnet" : "mainnet"
  ) as AnalyticsEvent["network"];

  // ── Attempt 1: Horizon (primary) ───────────────────────────────────────────
  try {
    // Fetch both invoke_host_function and upload_contract_wasm in parallel
    const [invokeRes, uploadRes] = await Promise.allSettled([
      horizonGet<HorizonOperationsResponse>(
        `${cfg.horizon}/operations?type=invoke_host_function&limit=${limit}&order=desc`,
      ),
      horizonGet<HorizonOperationsResponse>(
        `${cfg.horizon}/operations?type=upload_contract_wasm&limit=10&order=desc`,
      ),
    ]);

    const invokeRecords =
      invokeRes.status === "fulfilled"
        ? (invokeRes.value._embedded?.records ?? [])
        : [];
    const uploadRecords =
      uploadRes.status === "fulfilled"
        ? (uploadRes.value._embedded?.records ?? [])
        : [];

    // Merge and sort by created_at descending
    const merged = [...invokeRecords, ...uploadRecords]
      .sort(
        (a, b) =>
          new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      )
      .slice(0, limit);

    if (merged.length > 0) {
      return merged.map(
        (r): AnalyticsEvent => ({
          id: r.id,
          event_type: opTypeToEventType(r.type, r.function),
          contract_id: r.contract_id ?? r.transaction_hash,
          user_address: r.source_account,
          network: internalNetwork,
          metadata: {
            name: r.contract_id ? shortenId(r.contract_id) : undefined,
            function: r.function,
            tx_hash: r.transaction_hash,
          },
          created_at: r.created_at,
        }),
      );
    }
  } catch {
    // Horizon failed → try Stellar Expert
  }

  // ── Attempt 2: Stellar Expert operations (fallback) ────────────────────────
  interface ExpertOp {
    id: string;
    type: number;
    paging_token: string;
    transaction_hash: string;
    source_account?: string;
    created_at: string;
    contract_id?: string;
    function?: string;
  }
  interface ExpertOpsResponse {
    _embedded: { records: ExpertOp[] };
  }

  const url = `${cfg.expert}/operations?limit=${limit}&order=desc`;
  const data = await expertGet<ExpertOpsResponse>(url);
  const records = data?._embedded?.records ?? [];

  return records.map(
    (r): AnalyticsEvent => ({
      id: r.id,
      event_type: opTypeToEventType(
        r.type === 24 ? "upload_contract_wasm" : "invoke_host_function",
        r.function,
      ),
      contract_id: r.contract_id ?? r.transaction_hash,
      user_address: r.source_account ?? "unknown",
      network: internalNetwork,
      metadata: {
        name: r.contract_id ? shortenId(r.contract_id) : undefined,
        function: r.function,
        tx_hash: r.transaction_hash,
      },
      created_at: r.created_at,
    }),
  );
}

// ─── Utility ──────────────────────────────────────────────────────────────────

function shortenId(id: string): string {
  if (id.length <= 12) return id;
  return `${id.slice(0, 6)}…${id.slice(-4)}`;
}
