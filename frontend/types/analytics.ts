export type TimePeriod = "7d" | "30d" | "90d" | "all-time";

export interface SearchTrendPoint {
  date: string;
  searches: number;
  uniqueTerms: number;
}

export interface TopSearchTerm {
  term: string;
  count: number;
  growth: number; // percentage change vs prior period
}

export interface SankeyNode {
  id: string;
  name: string;
  category: "entry" | "search" | "filter" | "contract" | "action";
}

export interface SankeyLink {
  source: string;
  target: string;
  value: number;
}

export interface DiscoveryPaths {
  nodes: SankeyNode[];
  links: SankeyLink[];
}

export interface FunnelStage {
  stage: string;
  users: number;
  percentage: number;
}

export interface CategoryAnalytic {
  category: string;
  searches: number;
  views: number;
  deployments: number;
}

export interface NetworkRegionPoint {
  network: string;
  region: string;
  count: number;
  percentage: number;
}

export interface AnalyticsResponse {
  searchTrends: SearchTrendPoint[];
  topSearchTerms: TopSearchTerm[];
  discoveryPaths: DiscoveryPaths;
  engagementFunnel: FunnelStage[];
  categoryPopularity: CategoryAnalytic[];
  networkDistribution: NetworkRegionPoint[];
}
