import type { Contract } from "./api";
import type { VerificationStatus } from "../types/verification";

interface AbiFunctionLike {
  name: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isAbiFunctionLike(value: unknown): value is AbiFunctionLike {
  return (
    isRecord(value) &&
    typeof value.name === "string" &&
    value.name.trim().length > 0
  );
}

/**
 * Extracts the first ABI method names from an ABI payload returned by the backend.
 *
 * The backend returns `{ abi: { functions: [...] } }`, but we keep this defensive so the
 * quick-view modal can degrade gracefully when ABI data is absent or malformed.
 */
export function extractAbiMethodNames(abi: unknown, limit = 5): string[] {
  if (!isRecord(abi) || !Array.isArray(abi.functions)) {
    return [];
  }

  return abi.functions
    .filter(isAbiFunctionLike)
    .map((fn) => fn.name.trim())
    .slice(0, limit);
}

/**
 * Maps the contract summary verification flag onto the frontend badge status.
 */
export function getQuickViewVerificationStatus(
  contract: Pick<Contract, "is_verified">,
): VerificationStatus {
  return contract.is_verified ? "approved" : "submitted";
}
