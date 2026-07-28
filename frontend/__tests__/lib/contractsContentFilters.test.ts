import {
  buildContractsApiParams,
  getInitialFilters,
  type ContractsUiFilters,
} from "../../app/contracts/contracts-content";

function makeFilters(
  overrides: Partial<ContractsUiFilters> = {},
): ContractsUiFilters {
  return {
    query: "",
    categories: [],
    languages: [],
    tags: [],
    author: "",
    networks: [],
    verified_only: false,
    sort_by: "created_at",
    sort_order: "desc",
    page: 1,
    page_size: 12,
    ...overrides,
  };
}

test("getInitialFilters parses multi-select category and network params", () => {
  const searchParams = new URLSearchParams(
    "query=amm&category=DeFi&category=NFT&network=mainnet&network=testnet&verified_only=true&page=3",
  );

  const filters = getInitialFilters(searchParams);

  expect(filters.query).toBe("amm");
  expect(filters.categories).toEqual(["DeFi", "NFT"]);
  expect(filters.networks).toEqual(["mainnet", "testnet"]);
  expect(filters.verified_only).toBe(true);
  expect(filters.page).toBe(3);
});

test("buildContractsApiParams forwards selected filters using API field names", () => {
  const params = buildContractsApiParams(
    makeFilters({
      query: "vault",
      categories: ["DeFi", "Governance"],
      networks: ["mainnet", "testnet"],
      tags: ["amm"],
      author: "GABC",
      verified_only: true,
      sort_by: "popularity",
      sort_order: "asc",
      page: 2,
    }),
  );

  expect(params).toMatchObject({
    query: "vault",
    categories: ["DeFi", "Governance"],
    networks: ["mainnet", "testnet"],
    tags: ["amm"],
    author: "GABC",
    verified_only: true,
    sort_by: "popularity",
    sort_order: "asc",
    page: 2,
    page_size: 12,
  });

  expect((params as Record<string, unknown>).keyword).toBeUndefined();
});
