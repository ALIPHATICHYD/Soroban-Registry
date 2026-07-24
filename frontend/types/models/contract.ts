import { Network, NetworkConfig } from "./network";
import { VerificationLevel } from "../verification";

/**
 * Contract related types for Soroban Registry
 */

export interface Contract {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: Network;
  is_verified: boolean;
  verification_level?: VerificationLevel;
  category?: string;
  tags: string[];
  popularity_score?: number;
  downloads?: number;
  average_rating?: number;
  avg_rating?: number;
  review_count?: number;
  deployment_count?: number;
  interaction_count?: number;
  favorites_count?: number;
  relevance_score?: number;
  logo_url?: string;
  created_at: string;
  updated_at: string;
  verified_at?: string;
  last_accessed_at?: string;
  is_maintenance?: boolean;
  logical_id?: string;
  network_configs?: Record<Network, NetworkConfig>;
}

/** GET /contracts/:id response when ?network= is used (Issue #43) */
export interface ContractGetResponse extends Contract {
  current_network?: Network;
  network_config?: NetworkConfig;
}

export interface ContractHealth {
  contract_id: string;
  status: "healthy" | "warning" | "critical";
  last_activity: string;
  security_score: number;
  audit_date?: string;
  total_score: number;
  recommendations: string[];
  updated_at: string;
}

export interface ContractVersion {
  id: string;
  contract_id: string;
  version: string;
  wasm_hash: string;
  source_url?: string;
  commit_hash?: string;
  release_notes?: string;
  change_notes?: string;
  is_revert?: boolean;
  reverted_from?: string;
  created_at: string;
}

export interface VersionFieldDiff {
  field: string;
  from_value: unknown;
  to_value: unknown;
}

export interface VersionCompareResponse {
  contract_id: string;
  from_version: ContractVersion;
  to_version: ContractVersion;
  differences: VersionFieldDiff[];
  wasm_changed: boolean;
}

export interface RevertVersionRequest {
  change_notes?: string;
  admin_id: string;
}

export interface ContractAbiResponse {
  abi: unknown;
}

export interface ContractChangelogEntry {
  version: string;
  created_at: string;
  commit_hash?: string;
  source_url?: string;
  release_notes?: string;
  breaking: boolean;
  breaking_changes: string[];
}

export interface ContractChangelogResponse {
  contract_id: string;
  entries: ContractChangelogEntry[];
}

export interface DependencyTreeNode {
  contract_id: string;
  name: string;
  current_version: string;
  constraint_to_parent: string;
  dependencies: DependencyTreeNode[];
}

export type MaturityLevel = "alpha" | "beta" | "stable" | "mature" | "legacy";

export interface ContractSearchParams {
  query?: string;
  contract_id?: string;
  network?: Network;
  networks?: Network[];
  verified_only?: boolean;
  favorites_only?: boolean;
  favorites_list?: string[];
  category?: string;
  categories?: string[];
  language?: string;
  languages?: string[];
  author?: string;
  tags?: string[];
  maturity?: MaturityLevel;
  page?: number;
  page_size?: number;
  sort_by?:
    | "name"
    | "created_at"
    | "updated_at"
    | "popularity"
    | "deployments"
    | "interactions"
    | "relevance"
    | "downloads"
    | "rating";
  sort_order?: "asc" | "desc";
  date_from?: string;
  date_to?: string;
}

export type DeprecationStatus = "active" | "deprecated" | "retired";

export interface DeprecationInfo {
  contract_id: string;
  status: DeprecationStatus;
  deprecated_at?: string | null;
  retirement_at?: string | null;
  replacement_contract_id?: string | null;
  migration_guide_url?: string | null;
  notes?: string | null;
  days_remaining?: number | null;
  dependents_notified: number;
}

export interface ContractInteraction {
  contract_id: string;
  method: string;
  caller: string;
  args: Record<string, unknown>;
  result: unknown;
  ledger_sequence: number;
  transaction_hash: string;
  timestamp: string;
}

export interface InteractionsQueryParams {
  method?: string;
  caller?: string;
  account?: string;
  from_ledger?: number;
  to_ledger?: number;
  from_date?: string;
  to_date?: string;
  from_timestamp?: string;
  to_timestamp?: string;
  limit?: number;
  offset?: number;
}

export interface InteractionsListResponse {
  interactions: ContractInteraction[];
  total: number;
  limit: number;
  offset: number;
}

export interface ContractInteractionResponse {
  contract_id: string;
  network: Network;
  total_count: number;
  recent_interactions: ContractInteraction[];
  top_methods: Array<{ method: string; count: number }>;
  top_callers: Array<{ caller: string; count: number }>;
}

export type PublishRequest = {
  contract_id: string;
  publisher_id: string;
  network: Network;
  wasm_hash: string;
  name?: string;
  description?: string;
  category?: string;
  tags?: string[];
  logo_url?: string;
};
