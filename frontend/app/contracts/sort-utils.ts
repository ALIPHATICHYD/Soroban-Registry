import type { Contract } from "@/lib/api";

export type SortBy = "created_at" | "updated_at" | "popularity" | "relevance";
export type SortOrder = "asc" | "desc";

export interface SortPreference {
  sort_by: SortBy;
  sort_order: SortOrder;
}

export const CONTRACT_SORT_PREFERENCE_KEY = "contracts-sort-preference";

export const DEFAULT_SORT_PREFERENCE: SortPreference = {
  sort_by: "created_at",
  sort_order: "desc",
};

function isSortBy(value: string | null | undefined): value is SortBy {
  return (
    value === "created_at" ||
    value === "updated_at" ||
    value === "popularity" ||
    value === "relevance"
  );
}

export function normalizeSortBy(
  value: string | null | undefined,
  hasQuery = false,
): SortBy {
  if (isSortBy(value)) return value;
  return hasQuery ? "relevance" : DEFAULT_SORT_PREFERENCE.sort_by;
}

export function normalizeSortOrder(
  value: string | null | undefined,
): SortOrder {
  return value === "asc" || value === "desc"
    ? value
    : DEFAULT_SORT_PREFERENCE.sort_order;
}

export function readStoredSortPreference(
  storage?: Pick<Storage, "getItem"> | null,
): SortPreference | null {
  if (!storage) return null;

  try {
    const raw = storage.getItem(CONTRACT_SORT_PREFERENCE_KEY);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as Partial<SortPreference>;
    if (!isSortBy(parsed.sort_by ?? null)) return null;

    return {
      sort_by: parsed.sort_by ?? DEFAULT_SORT_PREFERENCE.sort_by,
      sort_order: normalizeSortOrder(parsed.sort_order),
    };
  } catch {
    return null;
  }
}

export function persistSortPreference(
  preference: SortPreference,
  storage?: Pick<Storage, "setItem"> | null,
): void {
  if (!storage) return;

  storage.setItem(CONTRACT_SORT_PREFERENCE_KEY, JSON.stringify(preference));
}

export function resolveInitialSortPreference(
  searchParams: URLSearchParams,
  storage?: Pick<Storage, "getItem"> | null,
): SortPreference {
  const query = searchParams.get("query") || searchParams.get("q") || "";
  const stored = readStoredSortPreference(storage);
  const urlSortBy = searchParams.get("sort_by");
  const urlSortOrder = searchParams.get("sort_order");

  return {
    sort_by: normalizeSortBy(urlSortBy ?? stored?.sort_by, Boolean(query)),
    sort_order: normalizeSortOrder(urlSortOrder ?? stored?.sort_order),
  };
}

function getNumericValue(contract: Contract, keys: string[]): number {
  for (const key of keys) {
    const value = (contract as Contract & Record<string, unknown>)[key];
    if (typeof value === "number" && Number.isFinite(value)) {
      return value;
    }
  }

  return 0;
}

function compareText(a: string, b: string) {
  return a.localeCompare(b, undefined, { sensitivity: "base" });
}

export function sortContracts(
  contracts: Contract[],
  preference: SortPreference,
): Contract[] {
  const direction = preference.sort_order === "asc" ? 1 : -1;

  return [...contracts].sort((a, b) => {
    let comparison = 0;

    switch (preference.sort_by) {
      case "popularity":
        comparison =
          getNumericValue(a, [
            "popularity_score",
            "interaction_count",
            "deployment_count",
          ]) -
          getNumericValue(b, [
            "popularity_score",
            "interaction_count",
            "deployment_count",
          ]);
        break;
      case "updated_at":
        comparison =
          new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime();
        break;
      case "relevance":
        comparison =
          getNumericValue(a, ["relevance_score", "popularity_score"]) -
          getNumericValue(b, ["relevance_score", "popularity_score"]);
        break;
      case "created_at":
      default:
        comparison =
          new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
        break;
    }

    if (comparison === 0) {
      comparison = compareText(a.name, b.name);
    }

    return comparison * direction;
  });
}
