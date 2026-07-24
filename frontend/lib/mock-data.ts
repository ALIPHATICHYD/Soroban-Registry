import {
  Contract,
  ContractExample,
  ContractVersion,
  DependencyTreeNode,
} from "./api";

export const MOCK_CONTRACTS: Contract[] = [
  {
    id: "c1",
    contract_id: "CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J",
    wasm_hash:
      "709e80c88487f2481e33845a0e9695d436a5a9c9f4c3d82a5c2d1b7a2d6e3f4a", // Random hash
    name: "Hello World Oracle",
    description:
      "A simple Oracle contract that allows storing and retrieving a greeting value. Perfect for beginners to understand Soroban storage and invocation.",
    publisher_id: "pub1",
    network: "testnet",
    is_verified: true,
    category: "oracle",
    tags: ["oracle", "storage", "educational"],
    created_at: new Date(Date.now() - 86400000 * 10).toISOString(), // 10 days ago
    updated_at: new Date(Date.now() - 86400000 * 2).toISOString(),
  },
  {
    id: "c2",
    contract_id: "CA45K6K66L45L6L45L6L45L6L45L6L45L6L45L6L45L6L45L",
    wasm_hash:
      "a1b2c3d4e5f678901234567890abcdef1234567890abcdef1234567890abcdef",
    name: "Liquidity Pool",
    description:
      "Standard AMM liquidity pool implementation supporting token swaps and liquidity provision.",
    publisher_id: "pub1",
    network: "testnet",
    is_verified: true,
    category: "defi",
    tags: ["defi", "amm", "swap"],
    created_at: new Date(Date.now() - 86400000 * 20).toISOString(),
    updated_at: new Date(Date.now() - 86400000 * 5).toISOString(),
  },
  {
    id: "c3",
    contract_id: "CB34M5M55N34N5N34N5N34N5N34N5N34N5N34N5N34N5N34N",
    wasm_hash:
      "f0e1d2c3b4a59687091234567890abcdef1234567890abcdef1234567890abcdef",
    name: "Voting DAO",
    description:
      "Governance contract for decentralized decision making with proposal creation and weighted voting.",
    publisher_id: "pub2",
    network: "mainnet",
    is_verified: true,
    category: "dao",
    tags: ["dao", "governance", "voting"],
    created_at: new Date(Date.now() - 86400000 * 30).toISOString(),
    updated_at: new Date(Date.now() - 86400000 * 15).toISOString(),
  },
];

export const MOCK_VERSIONS: Record<string, ContractVersion[]> = {
  c1: [
    {
      id: "v1-1",
      contract_id: "c1",
      version: "1.0.0",
      wasm_hash:
        "709e80c88487f2481e33845a0e9695d436a5a9c9f4c3d82a5c2d1b7a2d6e3f4a",
      commit_hash: "a1b2c3d",
      created_at: new Date(Date.now() - 86400000 * 10).toISOString(),
    },
  ],
  c2: [
    {
      id: "v2-1",
      contract_id: "c2",
      version: "1.0.0",
      wasm_hash:
        "a1b2c3d4e5f678901234567890abcdef1234567890abcdef1234567890abcdef",
      created_at: new Date(Date.now() - 86400000 * 20).toISOString(),
    },
  ],
  c3: [
    {
      id: "v3-1",
      contract_id: "c3",
      version: "1.0.0",
      wasm_hash:
        "f0e1d2c3b4a59687091234567890abcdef1234567890abcdef1234567890abcdef",
      created_at: new Date(Date.now() - 86400000 * 30).toISOString(),
    },
  ],
};

export const MOCK_EXAMPLES: Record<string, ContractExample[]> = {
  // Linking to "Hello World Oracle" ID 'c1' AND the contract_id 'CC...'
  // In a real app we'd likely look up by primary key (UUID), but frontend often uses the ID from the route.
  // The route /contracts/[id] usually passes the UUID or the hash. Let's assume it passes the UUID 'c1' for our mock navigation.
  c1: [
    {
      id: "mock-1",
      contract_id: "c1",
      title: "Initialize Contract",
      description:
        "How to initialize the contract client and invoke the hello function.",
      category: "basic",
      rating_up: 15,
      rating_down: 2,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_js: `import { Contract, Networks } from "@stellar/stellar-sdk";

const contractId = "CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J";
const contract = new Contract(contractId);

// Simulate the "hello" function
console.log("Contract initialized:", contractId);
console.log("Ready to invoke methods.");`,
      code_rust: `#![no_std]
use soroban_sdk::{contractimpl, Env, Symbol, symbol_short};

pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        symbol_short!("Hello")
    }
}`,
    },
    {
      id: "mock-2",
      contract_id: "c1",
      title: "Reading Storage",
      description: "How to read the greeting value from the ledger.",
      category: "advanced",
      rating_up: 8,
      rating_down: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_js: `// Assuming the contract has a "get_greeting" method
const op = contract.call("get_greeting");
console.log("built operation:", op);`,
      code_rust: `pub fn get_greeting(env: Env) -> Symbol {
    env.storage().instance().get(&symbol_short!("GREET")).unwrap()
}`,
    },
    {
      id: "mock-3",
      contract_id: "c1",
      title: "Cross-Contract Call",
      description: "Calling this contract from another contract.",
      category: "integration",
      rating_up: 22,
      rating_down: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_rust: `pub fn call_hello(env: Env, address: Address) {
    let client = HelloContractClient::new(&env, &address);
    client.hello(&symbol_short!("Dev"));
}`,
    },
  ],
};

export const MOCK_DEPENDENCIES: Record<string, DependencyTreeNode[]> = {
  c2: [
    {
      contract_id: "CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J",
      name: "Hello World Oracle",
      current_version: "1.0.0",
      constraint_to_parent: "^1.0.0",
      dependencies: [],
    },
  ],
  c3: [
    {
      contract_id: "CA45K6K66L45L6L45L6L45L6L45L6L45L6L45L6L45L6L45L",
      name: "Liquidity Pool",
      current_version: "1.0.0",
      constraint_to_parent: ">=1.0.0",
      dependencies: [
        {
          contract_id: "CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J",
          name: "Hello World Oracle",
          current_version: "1.0.0",
          constraint_to_parent: "^1.0.0",
          dependencies: [],
        },
      ],
    },
  ],
};

import { Template } from "./api";

export const MOCK_TEMPLATES: Template[] = [
  {
    id: "tmpl-1",
    slug: "token",
    name: "Fungible Token",
    description:
      "SEP-0041 compatible fungible token with mint, transfer, and allowance mechanics.",
    category: "token",
    version: "1.0.0",
    install_count: 843,
    parameters: [
      {
        name: "CONTRACT_NAME",
        type: "string",
        description: "Contract struct name",
      },
      { name: "SYMBOL", type: "string", description: "Token ticker symbol" },
      {
        name: "INITIAL_SUPPLY",
        type: "i128",
        default: "1000000",
        description: "Initial token supply",
      },
    ],
    created_at: new Date(Date.now() - 86400000 * 60).toISOString(),
  },
  {
    id: "tmpl-2",
    slug: "dex",
    name: "DEX Liquidity Pool",
    description:
      "Constant-product AMM with LP tokens, 0.3% swap fee, and add/remove liquidity.",
    category: "dex",
    version: "1.0.0",
    install_count: 412,
    parameters: [
      {
        name: "CONTRACT_NAME",
        type: "string",
        description: "Contract struct name",
      },
    ],
    created_at: new Date(Date.now() - 86400000 * 45).toISOString(),
  },
  {
    id: "tmpl-3",
    slug: "bridge",
    name: "Cross-Chain Bridge",
    description:
      "Lock-release bridge stub with nonce-based replay protection and admin-controlled release.",
    category: "bridge",
    version: "1.0.0",
    install_count: 198,
    parameters: [
      {
        name: "CONTRACT_NAME",
        type: "string",
        description: "Contract struct name",
      },
    ],
    created_at: new Date(Date.now() - 86400000 * 30).toISOString(),
  },
  {
    id: "tmpl-4",
    slug: "oracle",
    name: "Price Oracle",
    description:
      "Admin-controlled price oracle with per-asset price feeds and update timestamps.",
    category: "oracle",
    version: "1.0.0",
    install_count: 304,
    parameters: [
      {
        name: "CONTRACT_NAME",
        type: "string",
        description: "Contract struct name",
      },
      {
        name: "DECIMALS",
        type: "u32",
        default: "7",
        description: "Price decimal places",
      },
    ],
    created_at: new Date(Date.now() - 86400000 * 50).toISOString(),
  },
  {
    id: "tmpl-5",
    slug: "lending",
    name: "Lending Pool",
    description:
      "Collateralised lending with configurable LTV ratio, supply, borrow, and repay operations.",
    category: "lending",
    version: "1.0.0",
    install_count: 267,
    parameters: [
      {
        name: "CONTRACT_NAME",
        type: "string",
        description: "Contract struct name",
      },
      {
        name: "COLLATERAL_FACTOR",
        type: "u32",
        default: "75",
        description: "LTV % (0-100)",
      },
    ],
    created_at: new Date(Date.now() - 86400000 * 35).toISOString(),
  },
];
