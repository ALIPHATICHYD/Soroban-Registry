export interface ContractRecord {
  id: string;
  contract_id: string;
  wasm_hash: string;
  name: string;
  description: string | null;
  publisher_id: string;
  network: string;
  is_verified: boolean;
  category: string | null;
  tags: string[];
  slug: string;
  created_at: Date;
  updated_at: Date;
}

export interface CreateContractInput {
  contract_id: string;
  wasm_hash: string;
  name: string;
  description?: string;
  publisher_id: string;
  network: string;
  category?: string;
  tags?: string[];
  slug?: string;
}

export interface UpdateContractInput {
  wasm_hash?: string;
  name?: string;
  description?: string | null;
  category?: string | null;
  tags?: string[];
  is_verified?: boolean;
}

export interface ContractDependency {
  source_contract_db_id: string;
  target_contract_db_id: string;
  dependency_type: "static" | "call";
}

export interface CategorySuggestionInput {
  code?: string;
  metadata?: Record<string, unknown>;
}
