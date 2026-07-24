import type {
  PublisherResponse,
  ContractSummary,
  ActivityEvent,
} from "@/types";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "";
const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === "true";

// ---------------------------------------------------------------------------
// Real API call (primary path)
// ---------------------------------------------------------------------------

export async function getPublisher(
  address: string,
): Promise<PublisherResponse> {
  if (!USE_MOCKS) {
    const res = await fetch(
      `${API_URL}/api/publishers/${encodeURIComponent(address)}`,
    );
    if (!res.ok) {
      throw new Error(`Failed to fetch publisher: ${res.status}`);
    }
    return res.json();
  }

  // ---------------------------------------------------------------------------
  // Mock fallback (development only, gated behind NEXT_PUBLIC_USE_MOCKS)
  // ---------------------------------------------------------------------------
  await new Promise((resolve) => setTimeout(resolve, 800));

  const contracts = generateMockContracts(18);
  const activity = generateMockActivity(12);

  return {
    ...MOCK_PUBLISHER,
    address: address,
    avatarUrl: `https://api.dicebear.com/7.x/identicon/svg?seed=${address}`,
    contracts,
    activity,
  };
}

// ---------------------------------------------------------------------------
// Mock data generators (only used when USE_MOCKS === true)
// ---------------------------------------------------------------------------

// Mock data generator helper
const generateMockContracts = (count: number): ContractSummary[] => {
  return Array.from({ length: count }).map((_, i) => {
    const statusRand = Math.random();
    let status: "verified" | "failed" | "pending" = "verified";
    if (statusRand > 0.8) status = "failed";
    else if (statusRand > 0.6) status = "pending";

    return {
      id: `C${Math.random().toString(36).substring(2, 15).toUpperCase()}`,
      name: `Soroban Contract ${i + 1}`,
      description: `A sample Soroban smart contract for demonstration purposes. This contract handles specific logic for the dApp ecosystem.`,
      verificationStatus: status,
      deployedAt: new Date(
        Date.now() - Math.random() * 10000000000,
      ).toISOString(),
      tags: ["defi", "nft", "governance"].filter(() => Math.random() > 0.5),
    };
  });
};

const generateMockActivity = (count: number): ActivityEvent[] => {
  return Array.from({ length: count }).map(() => {
    const typeRand = Math.random();
    let type:
      | "verification_success"
      | "verification_failed"
      | "contract_published" = "contract_published";
    if (typeRand > 0.6) type = "verification_success";
    else if (typeRand > 0.4) type = "verification_failed";

    return {
      id: `act_${Math.random().toString(36).substring(2, 9)}`,
      type: type,
      contractName: `Soroban Contract ${Math.floor(Math.random() * 10) + 1}`,
      timestamp: new Date(
        Date.now() - Math.random() * 5000000000,
      ).toISOString(),
    };
  });
};

const MOCK_PUBLISHER: Omit<PublisherResponse, "contracts" | "activity"> = {
  address: "GBSX...2J4K", // This will be overwritten by the requested address
  displayName: "Stellar builder",
  bio: "Building decentralized applications on Soroban. Passionate about DeFi and DAO governance structures.",
  avatarUrl: "https://api.dicebear.com/7.x/identicon/svg?seed=stellar",
  website: "https://stellar.org",
  github: "https://github.com/stellar",
  verifiedContracts: 15,
  failedVerifications: 2,
  totalContracts: 18,
  createdAt: "2023-09-15T10:00:00Z",
};
