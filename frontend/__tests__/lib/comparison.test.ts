import { buildComparisonCsv } from "../../utils/export";
import {
  extractAbiMethods,
  toComparableContract,
  toneForMetricCell,
  diffMethodSets,
} from "../../utils/comparison";
import type { Contract } from "../../lib/api";

const baseContract: Contract = {
  id: "contract-1",
  contract_id: "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
  wasm_hash: "abc123",
  name: "Registry Token",
  description: "Token contract",
  publisher_id: "pub-1",
  network: "mainnet",
  is_verified: true,
  category: "token",
  tags: ["token", "defi"],
  created_at: "2026-04-01T00:00:00.000Z",
  updated_at: "2026-04-10T00:00:00.000Z",
};

test("extractAbiMethods finds method names from nested ABI objects", () => {
  const abi = {
    functions: [
      { name: "balance" },
      { name_scval: { symbol: "transfer" } },
      { function: "mint" },
    ],
  };

  expect(extractAbiMethods(abi)).toEqual(["balance", "mint", "transfer"]);
});

test("toComparableContract uses real contract metadata and version info", () => {
  const comparable = toComparableContract(baseContract, {
    versions: [
      { version: "1.0.0", created_at: "2026-04-01T00:00:00.000Z" },
      { version: "1.2.0", created_at: "2026-04-10T00:00:00.000Z" },
    ],
    abi: { functions: [{ name: "balance" }, { name: "transfer" }] },
    sourceCode: "pub fn transfer() {}",
  });

  expect(comparable.contractId).toBe(baseContract.contract_id);
  expect(comparable.latestVersion).toBe("1.2.0");
  expect(comparable.versionCount).toBe(2);
  expect(comparable.abiMethods).toEqual(["balance", "transfer"]);
  expect(comparable.sourceCode).toBe("pub fn transfer() {}");
});

test("toneForMetricCell marks differing categorical values", () => {
  expect(toneForMetricCell("network", "mainnet", ["mainnet", "testnet"])).toBe(
    "different",
  );
  expect(toneForMetricCell("verification_status", true, [true, false])).toBe(
    "best",
  );
  expect(toneForMetricCell("verification_status", false, [true, false])).toBe(
    "worst",
  );
});

test("diffMethodSets reports added and removed ABI methods", () => {
  expect(diffMethodSets(["balance", "transfer"], ["balance", "mint"])).toEqual({
    added: ["mint"],
    removed: ["transfer"],
  });
});

test("buildComparisonCsv exports comparison report rows", () => {
  const contracts = [
    toComparableContract(baseContract, {
      versions: [{ version: "1.0.0", created_at: "2026-04-01T00:00:00.000Z" }],
      abi: { functions: [{ name: "balance" }] },
    }),
    toComparableContract(
      {
        ...baseContract,
        id: "contract-2",
        name: "Registry Swap",
        network: "testnet",
        is_verified: false,
      },
      {
        versions: [
          { version: "2.0.0", created_at: "2026-04-12T00:00:00.000Z" },
        ],
        abi: { functions: [{ name: "swap" }] },
      },
    ),
  ];

  const csv = buildComparisonCsv(contracts, [
    { key: "network", label: "Network", getValue: (c) => c.network },
    {
      key: "latest_version",
      label: "Latest version",
      getValue: (c) => c.latestVersion,
    },
  ]);

  expect(csv).toContain("Attribute,Registry Token,Registry Swap");
  expect(csv).toContain("Network,mainnet,testnet");
  expect(csv).toContain("Latest version,1.0.0,2.0.0");
});
