import type { AnalyticsResponse, TimePeriod } from '@/types/analytics';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001';

const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === 'true';

export async function fetchAnalytics(period: TimePeriod): Promise<AnalyticsResponse> {
  if (!USE_MOCKS) {
    const res = await fetch(
      `${API_URL}/api/analytics/search?period=${encodeURIComponent(period)}`
    );
    if (!res.ok) {
      throw new Error(`Failed to fetch analytics: ${res.status}`);
    }
    return res.json();
  }

  const delay = Math.floor(Math.random() * 300) + 300;
  await new Promise((resolve) => setTimeout(resolve, delay));

  let days = 30;
  if (period === '7d') days = 7;
  if (period === '90d') days = 90;
  if (period === 'all-time') days = 180;

  return {
    searchTrends: generateSearchTrends(days),
    topSearchTerms: generateTopSearchTerms(),
    discoveryPaths: generateDiscoveryPaths(),
    engagementFunnel: generateEngagementFunnel(),
    categoryPopularity: generateCategoryPopularity(),
    networkDistribution: generateNetworkDistribution(),
  };
}

function rnd(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function generateSearchTrends(days: number) {
  const today = new Date();
  return Array.from({ length: days }, (_, i) => {
    const d = new Date(today);
    d.setDate(d.getDate() - (days - 1 - i));
    return {
      date: d.toISOString().split('T')[0],
      searches: rnd(80, 400),
      uniqueTerms: rnd(20, 120),
    };
  });
}

const SEARCH_TERMS = [
  'token', 'defi', 'swap', 'nft', 'dao', 'staking', 'liquidity',
  'bridge', 'oracle', 'multisig', 'vesting', 'auction', 'yield',
  'governance', 'lending', 'AMM', 'escrow', 'lottery', 'insurance',
  'identity', 'payroll', 'crowdfund', 'streaming', 'subscription',
];

function generateTopSearchTerms() {
  return SEARCH_TERMS.slice(0, 20).map((term) => ({
    term,
    count: rnd(50, 2000),
    growth: rnd(-30, 80),
  })).sort((a, b) => b.count - a.count);
}

function generateDiscoveryPaths() {
  const nodes = [
    { id: 'homepage', name: 'Homepage', category: 'entry' as const },
    { id: 'direct', name: 'Direct URL', category: 'entry' as const },
    { id: 'search_bar', name: 'Search Bar', category: 'search' as const },
    { id: 'browse', name: 'Browse All', category: 'search' as const },
    { id: 'filter_cat', name: 'Category Filter', category: 'filter' as const },
    { id: 'filter_net', name: 'Network Filter', category: 'filter' as const },
    { id: 'filter_tag', name: 'Tag Filter', category: 'filter' as const },
    { id: 'contract_view', name: 'Contract Detail', category: 'contract' as const },
    { id: 'deploy', name: 'Deploy', category: 'action' as const },
    { id: 'verify', name: 'Verify Source', category: 'action' as const },
    { id: 'copy_id', name: 'Copy Contract ID', category: 'action' as const },
  ];

  const links = [
    { source: 'homepage', target: 'search_bar', value: rnd(300, 600) },
    { source: 'homepage', target: 'browse', value: rnd(200, 400) },
    { source: 'direct', target: 'contract_view', value: rnd(150, 300) },
    { source: 'search_bar', target: 'filter_cat', value: rnd(200, 400) },
    { source: 'search_bar', target: 'contract_view', value: rnd(150, 350) },
    { source: 'browse', target: 'filter_cat', value: rnd(100, 250) },
    { source: 'browse', target: 'filter_net', value: rnd(80, 200) },
    { source: 'browse', target: 'filter_tag', value: rnd(60, 160) },
    { source: 'filter_cat', target: 'contract_view', value: rnd(200, 400) },
    { source: 'filter_net', target: 'contract_view', value: rnd(80, 180) },
    { source: 'filter_tag', target: 'contract_view', value: rnd(60, 140) },
    { source: 'contract_view', target: 'deploy', value: rnd(40, 120) },
    { source: 'contract_view', target: 'verify', value: rnd(30, 100) },
    { source: 'contract_view', target: 'copy_id', value: rnd(80, 200) },
  ];

  return { nodes, links };
}

function generateEngagementFunnel() {
  const visitors = rnd(8000, 15000);
  const searchers = Math.floor(visitors * (rnd(55, 70) / 100));
  const viewers = Math.floor(searchers * (rnd(40, 60) / 100));
  const interactors = Math.floor(viewers * (rnd(20, 40) / 100));
  const deployers = Math.floor(interactors * (rnd(10, 25) / 100));

  return [
    { stage: 'Visitors', users: visitors, percentage: 100 },
    { stage: 'Searched', users: searchers, percentage: Math.round((searchers / visitors) * 100) },
    { stage: 'Viewed Contract', users: viewers, percentage: Math.round((viewers / visitors) * 100) },
    { stage: 'Interacted', users: interactors, percentage: Math.round((interactors / visitors) * 100) },
    { stage: 'Deployed', users: deployers, percentage: Math.round((deployers / visitors) * 100) },
  ];
}

const CATEGORIES = ['DeFi', 'NFT', 'Gaming', 'Infrastructure', 'DAO', 'Wallet', 'Social', 'Tooling'];

function generateCategoryPopularity() {
  return CATEGORIES.map((category) => ({
    category,
    searches: rnd(100, 1200),
    views: rnd(200, 2500),
    deployments: rnd(10, 300),
  })).sort((a, b) => b.searches - a.searches);
}

const REGIONS = [
  { region: 'North America', network: 'Mainnet' },
  { region: 'Europe', network: 'Mainnet' },
  { region: 'Asia Pacific', network: 'Mainnet' },
  { region: 'Latin America', network: 'Testnet' },
  { region: 'Africa', network: 'Testnet' },
  { region: 'Middle East', network: 'Futurenet' },
  { region: 'Oceania', network: 'Futurenet' },
];

function generateNetworkDistribution() {
  const raw = REGIONS.map((r) => ({
    ...r,
    count: rnd(50, 800),
    percentage: 0,
  }));
  const total = raw.reduce((s, r) => s + r.count, 0);
  return raw.map((r) => ({ ...r, percentage: Math.round((r.count / total) * 100) }));
}

export async function fetchDashboardAnalytics(): Promise<unknown> {
  if (!USE_MOCKS) {
    const res = await fetch(`${API_URL}/api/analytics/dashboard`);
    if (!res.ok) {
      throw new Error(`Failed to fetch dashboard: ${res.status}`);
    }
    return res.json();
  }

  // Mock dashboard data
  const delay = Math.floor(Math.random() * 300) + 300;
  await new Promise((resolve) => setTimeout(resolve, delay));

  return {
    category_distribution: generateCategoryPopularity().map((c) => ({
      category: c.category,
      contract_count: c.deployments,
      total_views: c.views,
    })),
    network_usage: generateNetworkDistribution().map((n) => ({
      network: n.network.toLowerCase(),
      contract_count: n.count,
      verified_count: Math.floor(n.count * 0.7),
    })),
    deployment_trends: generateSearchTrends(30).map((t) => ({
      date: t.date,
      count: Math.floor(t.searches / 10),
    })),
    top_publishers: [],
    recent_additions: [],
  };
}

export async function fetchTrendingContracts(limit = 10): Promise<Record<string, unknown>[]> {
  if (!USE_MOCKS) {
    const res = await fetch(`${API_URL}/api/contracts/trending?limit=${limit}`);
    if (!res.ok) {
      throw new Error(`Failed to fetch trending: ${res.status}`);
    }
    const data = await res.json();
    return data.trending || [];
  }

  // Mock trending data
  const delay = Math.floor(Math.random() * 300) + 300;
  await new Promise((resolve) => setTimeout(resolve, delay));

  return [];
}
