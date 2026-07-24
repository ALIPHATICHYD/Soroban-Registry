import type { Contract } from "../../lib/api";
import {
  CONTRACT_SORT_PREFERENCE_KEY,
  normalizeSortBy,
  normalizeSortOrder,
  persistSortPreference,
  readStoredSortPreference,
  resolveInitialSortPreference,
  sortContracts,
} from "../../app/contracts/sort-utils";

function makeContract(
  overrides: Partial<Contract & Record<string, unknown>> = {},
): Contract {
  return {
    id: String(overrides.id ?? "contract-id"),
    contract_id: String(overrides.contract_id ?? "contract-address"),
    wasm_hash: "",
    name: String(overrides.name ?? "Contract"),
    publisher_id: "publisher-1",
    network: "mainnet",
    is_verified: true,
    tags: [],
    created_at: String(
      overrides.created_at ??
        new Date("2026-01-01T00:00:00.000Z").toISOString(),
    ),
    updated_at: String(
      overrides.updated_at ??
        new Date("2026-01-01T00:00:00.000Z").toISOString(),
    ),
    ...(overrides as Partial<Contract>),
  };
}

describe("contracts sort utils", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  test("resolveInitialSortPreference prefers persisted sort when URL does not override it", () => {
    persistSortPreference(
      { sort_by: "updated_at", sort_order: "asc" },
      window.localStorage,
    );

    const preference = resolveInitialSortPreference(
      new URLSearchParams(""),
      window.localStorage,
    );

    expect(preference).toEqual({ sort_by: "updated_at", sort_order: "asc" });
  });

  test("resolveInitialSortPreference prefers URL over persisted sort", () => {
    persistSortPreference(
      { sort_by: "updated_at", sort_order: "asc" },
      window.localStorage,
    );

    const preference = resolveInitialSortPreference(
      new URLSearchParams("sort_by=updated_at&sort_order=desc"),
      window.localStorage,
    );

    expect(preference).toEqual({ sort_by: "updated_at", sort_order: "desc" });
  });

  test("persistSortPreference writes a stable JSON payload", () => {
    persistSortPreference(
      { sort_by: "popularity", sort_order: "desc" },
      window.localStorage,
    );

    expect(window.localStorage.getItem(CONTRACT_SORT_PREFERENCE_KEY)).toBe(
      JSON.stringify({ sort_by: "popularity", sort_order: "desc" }),
    );
    expect(readStoredSortPreference(window.localStorage)).toEqual({
      sort_by: "popularity",
      sort_order: "desc",
    });
  });

  test("normalize helpers fall back to defaults", () => {
    expect(normalizeSortBy("invalid")).toBe("created_at");
    expect(normalizeSortBy("invalid", true)).toBe("relevance");
    expect(normalizeSortOrder("invalid")).toBe("desc");
  });

  test("sortContracts reorders by name", () => {
    const contracts = [
      makeContract({ id: "2", name: "Zeta" }),
      makeContract({ id: "1", name: "Alpha" }),
    ];

    expect(
      sortContracts(contracts, {
        sort_by: "created_at",
        sort_order: "asc",
      }).map((c) => c.name),
    ).toEqual(["Alpha", "Zeta"]);
  });

  test("sortContracts reorders by date", () => {
    const contracts = [
      makeContract({
        id: "older",
        name: "Older",
        created_at: "2026-01-01T00:00:00.000Z",
      }),
      makeContract({
        id: "newer",
        name: "Newer",
        created_at: "2026-03-01T00:00:00.000Z",
      }),
    ];

    expect(
      sortContracts(contracts, {
        sort_by: "created_at",
        sort_order: "desc",
      }).map((c) => c.id),
    ).toEqual(["newer", "older"]);
  });

  test("sortContracts reorders by popularity", () => {
    const contracts = [
      makeContract({ id: "low", name: "Low", popularity_score: 2 }),
      makeContract({ id: "high", name: "High", popularity_score: 10 }),
    ] as Array<Contract & { popularity_score?: number }>;

    expect(
      sortContracts(contracts, {
        sort_by: "popularity",
        sort_order: "desc",
      }).map((c) => c.id),
    ).toEqual(["high", "low"]);
  });

  test("sortContracts reorders by updated timestamp", () => {
    const contracts = [
      makeContract({
        id: "older",
        name: "Older",
        updated_at: "2026-01-01T00:00:00.000Z",
      }),
      makeContract({
        id: "newer",
        name: "Newer",
        updated_at: "2026-03-01T00:00:00.000Z",
      }),
    ];

    expect(
      sortContracts(contracts, {
        sort_by: "updated_at",
        sort_order: "desc",
      }).map((c) => c.id),
    ).toEqual(["newer", "older"]);
  });
});
