/**
 * Publisher related types
 */

export interface Publisher {
  id: string;
  stellar_address: string;
  username?: string;
  email?: string;
  github_url?: string;
  website?: string;
  avatar_url?: string;
  created_at: string;
}

export interface PublisherResponse {
  address: string;
  displayName: string;
  bio?: string;
  avatarUrl?: string;
  website?: string;
  github?: string;
  verifiedContracts: number;
  failedVerifications: number;
  totalContracts: number;
  createdAt: string;
  contracts: ContractSummary[];
  activity: ActivityEvent[];
}

export interface ContractSummary {
  id: string;
  name: string;
  description: string;
  verificationStatus: "verified" | "failed" | "pending";
  deployedAt: string;
  tags: string[];
}

export interface ActivityEvent {
  id: string;
  type: "verification_success" | "verification_failed" | "contract_published";
  contractName: string;
  timestamp: string;
}
