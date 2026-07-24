import { PoolClient, QueryResult, QueryResultRow } from "pg";
import { pool } from "../db.js";
import {
  ContractRecord,
  CreateContractInput,
  UpdateContractInput,
  ContractDependency,
} from "../contracts.models.js";

type DbExecutor = Pick<PoolClient, "query">;

export interface ContractsDataAccess {
  listContracts(
    limit?: number,
    offset?: number,
    client?: DbExecutor,
  ): Promise<ContractRecord[]>;
  getContractById(id: string, client?: DbExecutor): Promise<ContractRecord | null>;
  createContract(
    input: CreateContractInput,
    client?: DbExecutor,
  ): Promise<ContractRecord>;
  updateContract(
    id: string,
    input: UpdateContractInput,
    client?: DbExecutor,
  ): Promise<ContractRecord | null>;
  deleteContract(id: string, client?: DbExecutor): Promise<boolean>;
  getContractDependencies(id: string, client?: DbExecutor): Promise<ContractDependency[]>;
  getCategorizedContractsForTraining(limit?: number, client?: DbExecutor): Promise<
    Array<{
      id: string;
      category: string;
      name: string;
      description: string | null;
      tags: string[];
      source_code: string | null;
    }>
  >;
  withTransaction<T>(fn: (client: PoolClient) => Promise<T>): Promise<T>;
}

function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 240);
}

function resolveExecutor(client?: DbExecutor): DbExecutor {
  return client ?? pool;
}

async function runQuery<T extends QueryResultRow>(
  queryText: string,
  values: unknown[],
  client?: DbExecutor,
): Promise<QueryResult<T>> {
  const executor = resolveExecutor(client);
  return executor.query<T>(queryText, values);
}

/**
 * Lists contracts using pagination.
 */
export async function listContracts(
  limit = 50,
  offset = 0,
  client?: DbExecutor,
): Promise<ContractRecord[]> {
  const boundedLimit = Math.max(1, Math.min(limit, 100));
  const boundedOffset = Math.max(0, offset);
  const { rows } = await runQuery<ContractRecord>(
    `SELECT *
     FROM contracts
     ORDER BY created_at DESC
     LIMIT $1 OFFSET $2`,
    [boundedLimit, boundedOffset],
    client,
  );
  return rows;
}

/**
 * Fetches one contract by internal UUID id.
 */
export async function getContractById(
  id: string,
  client?: DbExecutor,
): Promise<ContractRecord | null> {
  const { rows } = await runQuery<ContractRecord>(
    `SELECT * FROM contracts WHERE id = $1`,
    [id],
    client,
  );
  return rows[0] ?? null;
}

/**
 * Creates a contract record.
 */
export async function createContract(
  input: CreateContractInput,
  client?: DbExecutor,
): Promise<ContractRecord> {
  const slug = input.slug?.trim() || slugify(input.name);
  const { rows } = await runQuery<ContractRecord>(
    `INSERT INTO contracts (
      contract_id, wasm_hash, name, description, publisher_id, network, category, tags, slug
    ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
    RETURNING *`,
    [
      input.contract_id,
      input.wasm_hash,
      input.name,
      input.description ?? null,
      input.publisher_id,
      input.network,
      input.category ?? null,
      input.tags ?? [],
      slug,
    ],
    client,
  );
  return rows[0];
}

/**
 * Updates mutable contract fields.
 */
export async function updateContract(
  id: string,
  input: UpdateContractInput,
  client?: DbExecutor,
): Promise<ContractRecord | null> {
  const fields: string[] = [];
  const values: unknown[] = [];
  let index = 1;

  if (input.wasm_hash !== undefined) {
    fields.push(`wasm_hash = $${index++}`);
    values.push(input.wasm_hash);
  }
  if (input.name !== undefined) {
    fields.push(`name = $${index++}`);
    values.push(input.name);
    fields.push(`slug = $${index++}`);
    values.push(slugify(input.name));
  }
  if (input.description !== undefined) {
    fields.push(`description = $${index++}`);
    values.push(input.description);
  }
  if (input.category !== undefined) {
    fields.push(`category = $${index++}`);
    values.push(input.category);
  }
  if (input.tags !== undefined) {
    fields.push(`tags = $${index++}`);
    values.push(input.tags);
  }
  if (input.is_verified !== undefined) {
    fields.push(`is_verified = $${index++}`);
    values.push(input.is_verified);
  }

  if (fields.length === 0) {
    return getContractById(id, client);
  }

  values.push(id);
  const { rows } = await runQuery<ContractRecord>(
    `UPDATE contracts
     SET ${fields.join(", ")}
     WHERE id = $${index}
     RETURNING *`,
    values,
    client,
  );
  return rows[0] ?? null;
}

/**
 * Deletes a contract by UUID id.
 */
export async function deleteContract(
  id: string,
  client?: DbExecutor,
): Promise<boolean> {
  const result = await runQuery(
    `DELETE FROM contracts WHERE id = $1`,
    [id],
    client,
  );
  return (result.rowCount ?? 0) > 0;
}

/**
 * Loads contract dependency edges for graph visualization.
 */
export async function getContractDependencies(
  id: string,
  client?: DbExecutor,
): Promise<ContractDependency[]> {
  const { rows } = await runQuery<ContractDependency>(
    `SELECT
      csd.contract_id AS source_contract_db_id,
      csd.dependency_contract_id AS target_contract_db_id,
      'static'::text AS dependency_type
     FROM contract_static_dependencies csd
     WHERE csd.contract_id = $1
       AND csd.dependency_contract_id IS NOT NULL
     UNION ALL
     SELECT
      ccd.caller_id AS source_contract_db_id,
      target.id AS target_contract_db_id,
      'call'::text AS dependency_type
     FROM contract_call_dependencies ccd
     JOIN contracts target
       ON target.contract_id = ccd.callee_contract_id
     WHERE ccd.caller_id = $1`,
    [id],
    client,
  );
  return rows;
}

/**
 * Loads categorized contracts used for classifier training.
 */
export async function getCategorizedContractsForTraining(
  limit = 1000,
  client?: DbExecutor,
): Promise<
  Array<{
    id: string;
    category: string;
    name: string;
    description: string | null;
    tags: string[];
    source_code: string | null;
  }>
> {
  const boundedLimit = Math.max(20, Math.min(limit, 5000));
  const { rows } = await runQuery<{
    id: string;
    category: string;
    name: string;
    description: string | null;
    tags: string[];
    source_code: string | null;
  }>(
    `SELECT
      c.id,
      c.category,
      c.name,
      c.description,
      c.tags,
      (
        SELECT v.source_code
        FROM verifications v
        WHERE v.contract_id = c.id
          AND v.source_code IS NOT NULL
        ORDER BY v.created_at DESC
        LIMIT 1
      ) AS source_code
     FROM contracts c
     WHERE c.category IS NOT NULL
       AND c.category <> ''
     ORDER BY c.updated_at DESC
     LIMIT $1`,
    [boundedLimit],
    client,
  );
  return rows;
}

/**
 * Runs operations inside a DB transaction.
 */
export async function withTransaction<T>(
  fn: (client: PoolClient) => Promise<T>,
): Promise<T> {
  const client = await pool.connect();
  try {
    await client.query("BEGIN");
    const result = await fn(client);
    await client.query("COMMIT");
    return result;
  } catch (error) {
    await client.query("ROLLBACK");
    throw error;
  } finally {
    client.release();
  }
}

/**
 * Creates the default contracts DAL backed by PostgreSQL.
 */
export function createContractsDataAccess(): ContractsDataAccess {
  return {
    listContracts,
    getContractById,
    createContract,
    updateContract,
    deleteContract,
    getContractDependencies,
    getCategorizedContractsForTraining,
    withTransaction,
  };
}

export const contractsDataAccess = createContractsDataAccess();
