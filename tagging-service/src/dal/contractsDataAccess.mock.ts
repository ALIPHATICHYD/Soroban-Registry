import { randomUUID } from "crypto";
import { PoolClient } from "pg";
import {
  ContractRecord,
  CreateContractInput,
  UpdateContractInput,
  ContractDependency,
} from "../contracts.models.js";
import { ContractsDataAccess } from "./contractsDataAccess.js";

function now(): Date {
  return new Date();
}

function toRecord(input: CreateContractInput): ContractRecord {
  const createdAt = now();
  return {
    id: randomUUID(),
    contract_id: input.contract_id,
    wasm_hash: input.wasm_hash,
    name: input.name,
    description: input.description ?? null,
    publisher_id: input.publisher_id,
    network: input.network,
    is_verified: false,
    category: input.category ?? null,
    tags: input.tags ?? [],
    slug: input.slug ?? input.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
    created_at: createdAt,
    updated_at: createdAt,
  };
}

/**
 * Creates an in-memory contracts DAL for unit tests.
 */
export function createMockContractsDataAccess(
  initial: ContractRecord[] = [],
): ContractsDataAccess {
  const contracts = new Map<string, ContractRecord>(
    initial.map((item) => [item.id, item]),
  );
  const dependencies = new Map<string, ContractDependency[]>();

  return {
    async listContracts(limit = 50, offset = 0) {
      return Array.from(contracts.values()).slice(offset, offset + limit);
    },

    async getContractById(id: string) {
      return contracts.get(id) ?? null;
    },

    async createContract(input: CreateContractInput) {
      const record = toRecord(input);
      contracts.set(record.id, record);
      return record;
    },

    async updateContract(id: string, input: UpdateContractInput) {
      const existing = contracts.get(id);
      if (!existing) {
        return null;
      }
      const updated: ContractRecord = {
        ...existing,
        ...input,
        updated_at: now(),
      };
      if (input.name) {
        updated.slug = input.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
      }
      contracts.set(id, updated);
      return updated;
    },

    async deleteContract(id: string) {
      return contracts.delete(id);
    },

    async getContractDependencies(id: string) {
      return dependencies.get(id) ?? [];
    },

    async getCategorizedContractsForTraining(limit = 1000) {
      return Array.from(contracts.values())
        .filter((item) => !!item.category)
        .slice(0, limit)
        .map((item) => ({
          id: item.id,
          category: item.category as string,
          name: item.name,
          description: item.description,
          tags: item.tags,
          source_code: null,
        }));
    },

    async withTransaction<T>(fn: (client: PoolClient) => Promise<T>) {
      // Mock transaction uses a no-op client because data is already in-memory.
      const mockClient = { query: async () => ({ rows: [], rowCount: 0 }) };
      return fn(mockClient as unknown as PoolClient);
    },
  };
}
