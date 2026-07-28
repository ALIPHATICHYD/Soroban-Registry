export type TimePeriod = "7d" | "30d" | "90d" | "all-time";

export interface StatsResponse {
  totalContracts: number;
  verifiedPercentage: number;
  totalPublishers: number;
  networkBreakdown: {
    network: string;
    contracts: number;
  }[];
  contractsByCategory: {
    category: string;
    count: number;
  }[];
  deploymentsTrend: {
    date: string;
    count: number;
  }[];
  topPublishers: {
    name: string;
    address: string;
    contractsDeployed: number;
  }[];
}
