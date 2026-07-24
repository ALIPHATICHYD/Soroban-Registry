"use client";

import { Suspense, useState, useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  Network,
  ContractVersion,
  ContractChangelogEntry,
} from "@/types";
import ExampleGallery from "@/components/ExampleGallery";
import {
  ArrowLeft,
  CheckCircle2,
  Globe,
  Tag,
  Search,
  History,
  Database,
  Layers,
  GitCompare,
  Share2,
} from "lucide-react";

import Link from "next/link";
import { useCopy } from "@/hooks/useCopy";
import CodeCopyButton from "@/components/CodeCopyButton";
import { useParams, useSearchParams } from "next/navigation";
import { useAnalytics } from "@/hooks/useAnalytics";
import FormalVerificationPanel from "@/components/FormalVerificationPanel";
import Navbar from "@/components/Navbar";
import MaintenanceBanner from "@/components/MaintenanceBanner";
import DeprecationBanner from "@/components/DeprecationBanner";
import ReleaseNotesPanel from "@/components/ReleaseNotesPanel";
import { useContractAutoRefresh } from "@/hooks/useContractAutoRefresh";
import ContractInteractionFlow from "@/components/contracts/ContractInteractionFlow";
import ContractAbiMethodExplorer from "@/components/contracts/ContractAbiMethodExplorer";
import VerificationBadge from "@/components/verification/VerificationBadge";

const NETWORKS: Network[] = ["mainnet", "testnet", "futurenet"];
const TAB_IDS = [
  "overview",
  "abi",
  "deployments",
  "interactions",
  "verification",
  "history",
] as const;
type TabId = (typeof TAB_IDS)[number];

// TODO: Replace with real API call when maintenance endpoint is available
const maintenanceStatus: { is_maintenance: boolean; current_window: null } = {
  is_maintenance: false,
  current_window: null,
};

function ContractDetailsContent() {
  const params = useParams<{ id?: string | string[] }>() ?? {};
  const searchParams = useSearchParams();
  const idParam = params.id;
  const id = Array.isArray(idParam) ? idParam[0] : idParam;
  const { copy: copyHeader, copied: copiedHeader } = useCopy();
  const { copy: copySidebar, copied: copiedSidebar } = useCopy();
  const { copy: copyShareLink, copied: copiedShareLink } = useCopy();
  const networkFromUrl = searchParams?.get("network") as Network | null;
  const [selectedNetwork, setSelectedNetwork] = useState<Network>(
    networkFromUrl && NETWORKS.includes(networkFromUrl)
      ? networkFromUrl
      : "mainnet",
  );
  const [activeTab, setActiveTab] = useState<TabId>("overview");
  const [tabSearch, setTabSearch] = useState("");

  const tabMeta: Record<
    TabId,
    { label: string; icon: React.ComponentType<{ className?: string }> }
  > = {
    overview: { label: "Overview", icon: Layers },
    abi: { label: "ABI", icon: Database },
    deployments: { label: "Deployments", icon: Globe },
    interactions: { label: "Interactions", icon: Share2 },
    verification: { label: "Verification", icon: CheckCircle2 },
    history: { label: "History", icon: History },
  };

  // Subscribe to real-time contract updates
  useContractAutoRefresh(id);

  const {
    data: contract,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["contract", id],
    queryFn: () => api.fetchContract(id!),
    enabled: !!id,
  });

  useQuery({
    queryKey: ["contract-dependencies", id],
    queryFn: () => api.fetchDependencyTree(id!),
    enabled: !!id && !!contract && activeTab === "overview",
  });

  const { data: versions = [] } = useQuery({
    queryKey: ["contract-versions", id],
    queryFn: () => api.fetchContractVersions(id!),
    enabled:
      !!id &&
      !!contract &&
      ["deployments", "history", "abi"].includes(activeTab),
  });

  const latestVersion = useMemo(() => {
    if (!versions.length) return null;
    return [...versions].sort(
      (a: ContractVersion, b: ContractVersion) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
    )[0];
  }, [versions]);

  const { data: abiResponse, isLoading: abiLoading } = useQuery({
    queryKey: ["contract-abi", id, latestVersion?.version],
    queryFn: () => api.fetchContractAbi(id!, latestVersion?.version),
    enabled: !!id && !!contract && activeTab === "abi",
  });

  const { data: analyticsData } = useQuery({
    queryKey: ["contract-analytics-summary", id],
    queryFn: () => api.fetchContractAnalytics(id!),
    enabled:
      !!id && !!contract && ["interactions", "deployments"].includes(activeTab),
  });

  const { data: interactionsData } = useQuery({
    queryKey: ["contract-interactions", id],
    queryFn: () => api.fetchContractInteractions(id!, { limit: 10, offset: 0 }),
    enabled: !!id && !!contract && activeTab === "interactions",
  });

  const { data: changelog } = useQuery({
    queryKey: ["contract-changelog", id],
    queryFn: () => api.fetchContractChangelog(id!),
    enabled: !!id && !!contract && activeTab === "history",
  });

  const sourceUrl = latestVersion?.source_url;

  const loweredSearch = tabSearch.trim().toLowerCase();
  const filteredVersions = useMemo(() => {
    if (!loweredSearch) return versions;
    return versions.filter((version) =>
      [
        version.version,
        version.wasm_hash,
        version.commit_hash,
        version.release_notes,
        version.source_url,
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(loweredSearch)),
    );
  }, [versions, loweredSearch]);

  const filteredHistory = useMemo(() => {
    const entries = changelog?.entries ?? [];
    if (!loweredSearch) return entries;
    return entries.filter((entry: ContractChangelogEntry) =>
      [
        entry.version,
        entry.commit_hash,
        entry.release_notes,
        entry.source_url,
        ...entry.breaking_changes,
      ]
        .filter(Boolean)
        .some((value) => String(value).toLowerCase().includes(loweredSearch)),
    );
  }, [changelog, loweredSearch]);

  const { logEvent } = useAnalytics();

  useEffect(() => {
    if (!error) return;
    logEvent("error_event", {
      source: "contract_details",
      contract_id: id,
      message: "Failed to load contract details",
    });
  }, [error, id, logEvent]);

  const { data: deprecationInfo } = useQuery({
    queryKey: ["contract-deprecation", id],
    queryFn: () => api.fetchDeprecationInfo(id!),
    enabled: !!id && !!contract,
  });

  if (!id) {
    return (
      <div className="min-h-screen bg-background text-foreground">
        <Navbar />
        <div className="max-w-4xl mx-auto px-4 py-10">
          <div className="rounded-2xl border border-border bg-card p-6">
            <div className="text-sm font-semibold text-foreground">
              Missing contract id
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              Open this page from the contracts list.
            </div>
            <div className="mt-4">
              <Link
                href="/contracts"
                className="inline-flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm font-semibold text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
              >
                Browse contracts
              </Link>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="animate-pulse space-y-8">
          <div className="h-8 bg-muted rounded w-1/3" />
          <div className="h-4 bg-muted rounded w-1/2" />
          <div className="h-64 bg-muted rounded-xl" />
        </div>
      </div>
    );
  }

  if (error || !contract) {
    return (
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="p-4 bg-red-500/10 border border-red-500/20 text-red-500 rounded-xl">
          Failed to load contract details
        </div>
      </div>
    );
  }

  const configForNetwork = contract.network_configs?.[selectedNetwork];
  const displayContractId =
    configForNetwork?.contract_id ?? contract.contract_id;
  const displayVerified = configForNetwork?.is_verified ?? contract.is_verified;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 animate-in fade-in duration-500">
      <Link
        href="/contracts"
        className="inline-flex items-center gap-2 text-muted-foreground hover:text-foreground mb-8 transition-colors"
      >
        <ArrowLeft className="w-4 h-4" />
        Back to contracts
      </Link>

      {/* Maintenance Banner */}
      {maintenanceStatus?.is_maintenance &&
        maintenanceStatus.current_window && (
          <MaintenanceBanner window={maintenanceStatus.current_window} />
        )}

      {/* Deprecation Banner */}
      {deprecationInfo && <DeprecationBanner info={deprecationInfo} />}

      {/* Header */}
      <div className="mb-12">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-4xl font-bold text-foreground mb-2">
              {contract.name}
            </h1>
            <div className="flex items-center gap-3 text-muted-foreground">
              <span className="flex items-center gap-2 font-mono bg-accent px-2 py-1 rounded-lg text-sm">
                <span>{displayContractId}</span>
                <CodeCopyButton
                  copied={copiedHeader}
                  onCopy={() =>
                    copyHeader(displayContractId, {
                      successEventName: "contract_address_copied",
                      failureEventName: "contract_address_copy_failed",
                      successMessage: "Contract address copied",
                      failureMessage: "Unable to copy contract address",
                      analyticsParams: {
                        contract_id: contract.id,
                        location: "header",
                      },
                    })
                  }
                  idleLabel="Copy"
                  copiedLabel="Copied"
                />
              </span>
              <button
                type="button"
                onClick={() => {
                  const url =
                    typeof window !== "undefined"
                      ? window.location.href
                      : `${contract.id}`;
                  copyShareLink(url, {
                    successEventName: "contract_share_link_copied",
                    failureEventName: "contract_share_link_copy_failed",
                    successMessage: "Contract link copied",
                    failureMessage: "Unable to copy contract link",
                    analyticsParams: { contract_id: contract.id },
                  });
                }}
                className="inline-flex items-center gap-1 rounded-lg border border-border bg-card px-2.5 py-1 text-xs font-semibold text-foreground hover:bg-accent transition-colors"
              >
                <Share2 className="w-3.5 h-3.5" />
                {copiedShareLink ? "Link Copied" : "Share"}
              </button>
              <VerificationBadge
                status={displayVerified ? "approved" : "unverified"}
                level={contract.verification_level}
                size="md"
              />
            </div>
          </div>

          {/* Network tabs */}
          <div className="flex gap-1 p-1 bg-accent rounded-xl w-fit">
            {NETWORKS.map((net) => {
              const hasConfig = !!contract.network_configs?.[net];
              return (
                <button
                  key={net}
                  type="button"
                  onClick={() => setSelectedNetwork(net)}
                  className={`px-4 py-2 rounded-lg text-sm font-medium capitalize transition-all ${
                    selectedNetwork === net
                      ? "bg-card text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  } ${!hasConfig ? "opacity-60" : ""}`}
                >
                  {net}
                </button>
              );
            })}
          </div>

          <div className="flex gap-2">
            <Link
              href={`/compare?contracts=${contract.id}`}
              className="inline-flex items-center gap-2 rounded-xl border border-border bg-card px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              <GitCompare className="h-4 w-4" />
              Compare
            </Link>
            <Link
              href={`/contracts/${id}/compatibility`}
              className="inline-flex items-center gap-2 rounded-xl border border-border bg-card px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              <GitCompare className="h-4 w-4" />
              Interoperability
            </Link>
          </div>
        </div>

        {contract.description && (
          <p className="text-xl text-muted-foreground max-w-3xl mb-6">
            {contract.description}
          </p>
        )}

        <div className="flex flex-wrap gap-2">
          {contract.tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-primary/10 text-primary text-sm font-medium"
            >
              <Tag className="w-3 h-3" />
              {tag}
            </span>
          ))}
        </div>
      </div>

      <div className="space-y-5">
        <div className="md:hidden">
          <label className="sr-only" htmlFor="contract-tab-select">
            Contract tab
          </label>
          <select
            id="contract-tab-select"
            className="w-full rounded-xl border border-border bg-card p-3 text-sm text-foreground"
            value={activeTab}
            onChange={(e) => {
              setActiveTab(e.target.value as TabId);
              setTabSearch("");
            }}
          >
            {TAB_IDS.map((tabId) => (
              <option key={tabId} value={tabId}>
                {tabMeta[tabId].label}
              </option>
            ))}
          </select>
        </div>

        <div className="hidden md:flex flex-wrap gap-2 p-2 rounded-2xl border border-border bg-card">
          {TAB_IDS.map((tabId) => {
            const Icon = tabMeta[tabId].icon;
            return (
              <button
                key={tabId}
                type="button"
                onClick={() => {
                  setActiveTab(tabId);
                  setTabSearch("");
                }}
                className={`inline-flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-medium transition-colors ${
                  activeTab === tabId
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-accent"
                }`}
              >
                <Icon className="w-4 h-4" />
                {tabMeta[tabId].label}
              </button>
            );
          })}
        </div>

        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="search"
            value={tabSearch}
            onChange={(e) => setTabSearch(e.target.value)}
            placeholder={`Search in ${tabMeta[activeTab].label}`}
            className="w-full rounded-xl border border-border bg-card pl-10 pr-3 py-2.5 text-sm text-foreground"
          />
        </div>

        {activeTab === "overview" && (
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2 space-y-6">
              <section>
                <ExampleGallery contractId={contract.id} />
              </section>

              <section className="bg-card rounded-2xl border border-border p-6">
                <h3 className="font-semibold text-foreground mb-4">
                  Contract Metadata
                </h3>
                <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
                  <div>
                    <dt className="text-muted-foreground">Name</dt>
                    <dd className="font-medium text-foreground">
                      {contract.name}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Author</dt>
                    <dd className="font-medium text-foreground break-all">
                      {contract.publisher_id}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Network</dt>
                    <dd className="font-medium text-foreground capitalize">
                      {selectedNetwork}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Category</dt>
                    <dd className="font-medium text-foreground">
                      {contract.category ?? "Uncategorized"}
                    </dd>
                  </div>
                  <div className="sm:col-span-2">
                    <dt className="text-muted-foreground">Description</dt>
                    <dd className="font-medium text-foreground">
                      {contract.description ?? "No description provided."}
                    </dd>
                  </div>
                  {sourceUrl && (
                    <div className="sm:col-span-2">
                      <dt className="text-muted-foreground">Links</dt>
                      <dd>
                        <a
                          href={sourceUrl}
                          target="_blank"
                          rel="noreferrer"
                          className="text-primary hover:underline break-all"
                        >
                          Source repository
                        </a>
                      </dd>
                    </div>
                  )}
                </dl>
              </section>
            </div>

            <div className="space-y-6">
              <div className="bg-card rounded-2xl border border-border p-6">
                <h3 className="font-semibold text-foreground mb-4">Key Info</h3>
                <dl className="space-y-3 text-sm">
                  <div>
                    <dt className="text-muted-foreground">Network</dt>
                    <dd className="font-medium text-foreground capitalize">
                      {selectedNetwork}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Address</dt>
                    <dd className="flex items-center justify-between gap-2 font-mono text-xs text-foreground break-all">
                      <span>{displayContractId}</span>
                      <CodeCopyButton
                        copied={copiedSidebar}
                        onCopy={() =>
                          copySidebar(displayContractId, {
                            successEventName: "contract_address_copied",
                            failureEventName: "contract_address_copy_failed",
                            successMessage: "Contract address copied",
                            failureMessage: "Unable to copy contract address",
                            analyticsParams: {
                              contract_id: contract.id,
                              location: "sidebar",
                            },
                          })
                        }
                        idleLabel="Copy"
                        copiedLabel="Copied"
                      />
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Published</dt>
                    <dd className="font-medium text-foreground">
                      {new Date(contract.created_at).toLocaleDateString()}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Last Updated</dt>
                    <dd className="font-medium text-foreground">
                      {new Date(contract.updated_at).toLocaleDateString()}
                    </dd>
                  </div>
                </dl>
              </div>

              <Link
                href={`/contracts/${contract.id}/api-docs`}
                className="flex items-center gap-3 w-full px-4 py-3 rounded-xl border border-border bg-card hover:bg-primary/5 hover:border-primary/30 text-muted-foreground hover:text-primary transition-all group"
              >
                <Globe className="w-5 h-5 text-muted-foreground group-hover:text-primary transition-colors" />
                <div>
                  <div className="text-sm font-medium">API Docs</div>
                  <div className="text-xs text-muted-foreground">
                    OpenAPI / Swagger UI
                  </div>
                </div>
              </Link>
            </div>
          </div>
        )}

        {activeTab === "interactions" && (
          <div className="space-y-6">
            <section className="bg-card rounded-2xl border border-border p-6">
              <div className="flex items-center justify-between mb-2">
                <h2 className="text-xl font-semibold text-foreground">
                  Interaction Flow Diagram
                </h2>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <span className="w-3 h-3 rounded-full bg-primary/20 border border-primary/50" />
                  <span>Interactive Flow</span>
                </div>
              </div>
              <p className="text-sm text-muted-foreground mb-6">
                Explore the cross-contract call graph centered on this contract.
                Zoom, pan, and filter to understand complex relationships.
              </p>
              <ContractInteractionFlow contractId={id} />
            </section>

            <section className="bg-card rounded-2xl border border-border p-6">
              <h3 className="text-lg font-semibold text-foreground mb-4">
                Interaction Statistics
              </h3>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 mb-5">
                <div className="rounded-xl border border-border p-3 bg-background">
                  <p className="text-xs text-muted-foreground uppercase tracking-wide">
                    Recent calls
                  </p>
                  <p className="text-2xl font-bold text-foreground">
                    {interactionsData?.total ?? 0}
                  </p>
                </div>
                <div className="rounded-xl border border-border p-3 bg-background">
                  <p className="text-xs text-muted-foreground uppercase tracking-wide">
                    Unique users
                  </p>
                  <p className="text-2xl font-bold text-foreground">
                    {analyticsData?.interactors.unique_count ?? 0}
                  </p>
                </div>
                <div className="rounded-xl border border-border p-3 bg-background">
                  <p className="text-xs text-muted-foreground uppercase tracking-wide">
                    Error rate
                  </p>
                  <p className="text-2xl font-bold text-foreground">N/A</p>
                </div>
              </div>

              <div className="space-y-2">
                {(interactionsData?.items ?? []).map((item) => (
                  <div
                    key={item.id}
                    className="rounded-lg border border-border bg-background px-3 py-2 text-sm"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <span className="font-mono text-xs text-foreground">
                        {item.method ?? "unknown_method"}
                      </span>
                      <time className="text-xs text-muted-foreground">
                        {new Date(item.created_at).toLocaleString()}
                      </time>
                    </div>
                    <p className="text-xs text-muted-foreground break-all mt-1">
                      {item.account ?? "Unknown account"}
                    </p>
                  </div>
                ))}
                {(interactionsData?.items?.length ?? 0) === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No recent interactions found.
                  </p>
                )}
              </div>
            </section>
          </div>
        )}

        {activeTab === "abi" && (
          <section className="bg-card rounded-2xl border border-border p-6 space-y-6">
            <div className="flex items-center justify-between gap-4">
              <div>
                <h2 className="text-xl font-semibold text-foreground">
                  ABI Method Explorer
                </h2>
                <p className="text-sm text-muted-foreground mt-1">
                  Browse contract methods, input parameters, simulate calls, and
                  copy SDK snippets.
                </p>
              </div>
              {abiResponse?.abi != null && (
                <details className="shrink-0">
                  <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground transition-colors select-none">
                    View raw JSON
                  </summary>
                  <div className="absolute right-6 z-20 mt-2 w-[min(600px,90vw)] max-h-96 overflow-auto rounded-xl border border-border bg-zinc-950 p-4 shadow-2xl">
                    <pre className="text-[11px] leading-5 text-zinc-300 font-mono">
                      {JSON.stringify(abiResponse.abi, null, 2)}
                    </pre>
                  </div>
                </details>
              )}
            </div>

            {abiLoading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <div
                    key={i}
                    className="h-14 animate-pulse bg-muted rounded-xl"
                  />
                ))}
              </div>
            ) : (
              <ContractAbiMethodExplorer
                abi={abiResponse?.abi}
                contractId={displayContractId}
              />
            )}
          </section>
        )}

        {activeTab === "deployments" && (
          <section className="bg-card rounded-2xl border border-border p-6 space-y-6">
            <h2 className="text-xl font-semibold text-foreground">
              Deployments
            </h2>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="rounded-xl border border-border p-4 bg-background">
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Total deployments
                </p>
                <p className="text-2xl font-bold text-foreground">
                  {analyticsData?.deployments.count ?? 0}
                </p>
              </div>
              <div className="rounded-xl border border-border p-4 bg-background">
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Unique users
                </p>
                <p className="text-2xl font-bold text-foreground">
                  {analyticsData?.deployments.unique_users ?? 0}
                </p>
              </div>
              <div className="rounded-xl border border-border p-4 bg-background">
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Versions
                </p>
                <p className="text-2xl font-bold text-foreground">
                  {versions.length}
                </p>
              </div>
            </div>

            <div className="space-y-3">
              <h3 className="text-sm font-semibold text-foreground uppercase tracking-wide">
                Network Deployments
              </h3>
              <div className="space-y-3">
                {Object.entries(contract.network_configs ?? {}).map(
                  ([network, config]) => (
                    <div
                      key={network}
                      className="rounded-xl border border-border p-4 bg-background"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <p className="font-semibold text-foreground capitalize">
                          {network}
                        </p>
                        <time className="text-xs text-muted-foreground">
                          {new Date(contract.updated_at).toLocaleString()}
                        </time>
                      </div>
                      <p className="text-xs text-muted-foreground mt-1 font-mono break-all">
                        {config.contract_id}
                      </p>
                    </div>
                  ),
                )}
              </div>

              <h3 className="text-sm font-semibond text-foreground uppercase tracking-wide">
                Deployment Timeline
              </h3>
              <div className="space-y-3">
                {filteredVersions.map((version) => (
                  <div
                    key={version.id}
                    className="rounded-xl border border-border p-4 bg-background"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <p className="font-semibold text-foreground">
                        v{version.version}
                      </p>
                      <time className="text-xs text-muted-foreground">
                        {new Date(version.created_at).toLocaleString()}
                      </time>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1 font-mono break-all">
                      {version.wasm_hash}
                    </p>
                    {version.release_notes && (
                      <p className="text-sm text-muted-foreground mt-2">
                        {version.release_notes}
                      </p>
                    )}
                  </div>
                ))}
                {filteredVersions.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No deployment timeline entries match this search.
                  </p>
                )}
              </div>
            </div>
          </section>
        )}

        {activeTab === "verification" && (
          <section className="space-y-4">
            <div className="bg-card rounded-2xl border border-border p-6">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h2 className="text-xl font-semibold text-foreground">
                  Verification Status
                </h2>
                <div className="flex items-center gap-4">
                  <VerificationBadge
                    status={displayVerified ? "approved" : "unverified"}
                    level={contract.verification_level}
                    size="md"
                  />
                  {!displayVerified && (
                    <Link
                      href={`/verify-contract?id=${contract.id}`}
                      className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:brightness-110 transition-all"
                    >
                      Submit for Verification
                    </Link>
                  )}
                </div>
              </div>
              <dl className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
                {displayVerified ? (
                  <>
                    <div>
                      <dt className="text-muted-foreground">Auditor</dt>
                      <dd className="font-medium text-foreground">
                        Soroban Auto-Auditor
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">
                        Verification Date
                      </dt>
                      <dd className="font-medium text-foreground">
                        {contract.verified_at
                          ? new Date(contract.verified_at).toLocaleDateString()
                          : "Recent"}
                      </dd>
                    </div>
                  </>
                ) : (
                  <>
                    <div>
                      <dt className="text-muted-foreground">Status</dt>
                      <dd className="font-medium text-foreground">
                        Not submitted
                      </dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Progress</dt>
                      <dd className="font-medium text-foreground">0%</dd>
                    </div>
                  </>
                )}
                <div className="sm:col-span-2">
                  <dt className="text-muted-foreground">Report</dt>
<dd>
                    <Link
                      href="/verification-status"
                      className="text-primary hover:underline"
                    >
                      Open detailed verification report
                    </Link>
                  </dd>
                </div>
              </dl>
            </div>
            <FormalVerificationPanel contractId={contract.id} />
          </section>
        )}

        {activeTab === "history" && (
          <div className="space-y-6">
            <section className="bg-card rounded-2xl border border-border p-6 space-y-4">
              <div className="flex items-center justify-between gap-3">
                <h2 className="text-xl font-semibold text-foreground">
                  Update History
                </h2>
                <Link
                  href={`/contracts/${id}/diff`}
                  className="flex items-center gap-1.5 rounded-xl border border-border bg-background px-3 py-1.5 text-xs font-semibold text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                >
                  <GitCompare size={14} />
                  View code diff
                </Link>
              </div>
              <div className="space-y-3">
                {filteredHistory.map((entry: ContractChangelogEntry) => (
                  <article
                    key={`${entry.version}-${entry.created_at}`}
                    className="rounded-xl border border-border p-4 bg-background"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2 mb-1.5">
                      <p className="font-semibold text-foreground">
                        v{entry.version}
                      </p>
                      <time className="text-xs text-muted-foreground">
                        {new Date(entry.created_at).toLocaleString()}
                      </time>
                    </div>
                    {entry.release_notes && (
                      <p className="text-sm text-muted-foreground mb-2">
                        {entry.release_notes}
                      </p>
                    )}
                    {entry.breaking && (
                      <div className="rounded-lg bg-red-500/10 text-red-500 px-3 py-2 text-xs">
                        Breaking changes detected
                      </div>
                    )}
                    {entry.breaking_changes.length > 0 && (
                      <ul className="mt-2 list-disc ml-4 text-xs text-muted-foreground space-y-1">
                        {entry.breaking_changes.map((change) => (
                          <li key={change}>{change}</li>
                        ))}
                      </ul>
                    )}
                  </article>
                ))}
                {filteredHistory.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No history entries match this search.
                  </p>
                )}
              </div>
            </section>
            <ReleaseNotesPanel contractId={contract.id} />
          </div>
        )}
      </div>
    </div>
  );
}

export default function ContractPage() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />
      <Suspense fallback={null}>
        <ContractDetailsContent />
      </Suspense>
    </div>
  );
}
