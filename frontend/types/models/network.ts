/**
 * Network related types for Soroban Registry
 */

export type Network = "mainnet" | "testnet" | "futurenet";

export type NetworkStatus = "online" | "offline" | "degraded";

export interface NetworkEndpoints {
  rpc_url: string;
  health_url: string;
  explorer_url: string;
  friendbot_url?: string;
}

export interface NetworkInfo {
  id: string;
  name: string;
  network_type: Network;
  status: NetworkStatus;
  endpoints: NetworkEndpoints;
  last_checked_at: string;
  last_indexed_ledger_height?: number;
  last_indexed_at?: string;
  consecutive_failures: number;
  status_message?: string;
}

export interface NetworkListResponse {
  networks: NetworkInfo[];
  cached_at: string;
}

/** Per-network config (Issue #43) */
export interface NetworkConfig {
  contract_id: string;
  is_verified: boolean;
  min_version?: string;
  max_version?: string;
}
