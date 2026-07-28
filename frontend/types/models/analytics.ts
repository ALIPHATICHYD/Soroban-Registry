import { Network } from "./network";

/**
 * Analytics and Metrics related types
 */

export interface TimelineEntry {
  date: string;
  count: number;
}

export interface TopUser {
  address: string;
  count: number;
}

export interface InteractorStats {
  unique_count: number;
  top_users: TopUser[];
}

export interface DeploymentStats {
  count: number;
  unique_users: number;
  by_network: Record<string, number>;
}

export interface ContractAnalyticsResponse {
  contract_id: string;
  deployments: DeploymentStats;
  interactors: InteractorStats;
  timeline: TimelineEntry[];
}

export type AnalyticsEventType =
  | "contract_published"
  | "contract_verified"
  | "contract_deployed"
  | "version_created"
  | "contract_updated"
  | "publisher_created"
  | "search_click";

export interface AnalyticsEvent {
  id: string;
  event_type: AnalyticsEventType;
  contract_id: string;
  user_address: string | null;
  network: Network | null;
  metadata: Record<string, unknown> | null;
  created_at: string;
}

export interface ActivityFeedParams {
  cursor?: string;
  limit?: number;
  event_type?: AnalyticsEventType;
  contract_id?: string;
}

export interface ActivityFeedResponse {
  items: AnalyticsEvent[];
  total: number;
  limit: number;
  next_cursor: string | null;
}

export type CustomMetricType = "counter" | "gauge" | "histogram";

export interface MetricCatalogEntry {
  metric_name: string;
  metric_type: CustomMetricType;
  last_seen: string;
  sample_count: number;
}

export interface MetricSeriesPoint {
  bucket_start: string;
  bucket_end: string;
  sample_count: number;
  sum_value?: number;
  avg_value?: number;
  min_value?: number;
  max_value?: number;
  p50_value?: number;
  p95_value?: number;
  p99_value?: number;
}

export interface MetricSample {
  timestamp: string;
  value: number;
  unit?: string;
  metadata?: Record<string, unknown> | null;
}

export interface MetricSeriesResponse {
  contract_id: string;
  metric_name: string;
  metric_type: CustomMetricType | null;
  resolution: "hour" | "day" | "raw";
  points?: MetricSeriesPoint[];
  samples?: MetricSample[];
}

export interface AnalyticsResponse {
  searchTrends: Array<{
    date: string;
    searches: number;
    uniqueTerms: number;
  }>;
  topSearchTerms: Array<{
    term: string;
    count: number;
    growth: number;
  }>;
  discoveryPaths: {
    nodes: Array<{
      id: string;
      name: string;
      category: string;
    }>;
    links: Array<{
      source: string;
      target: string;
      value: number;
    }>;
  };
  engagementFunnel: Array<{
    stage: string;
    users: number;
    percentage: number;
  }>;
  categoryPopularity: Array<{
    category: string;
    searches: number;
    views: number;
    deployments: number;
  }>;
  networkDistribution: Array<{
    region: string;
    network: string;
    count: number;
    percentage: number;
  }>;
}
