"use client";

import React, { useState, useEffect, useMemo, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import type {
  ContractSearchParams,
  Contract,
  SemanticContractSearchResponse,
} from "@/types";
import { api } from "@/lib/api";
import ContractCard from "@/components/ContractCard";
import ContractCardSkeleton from "@/components/ContractCardSkeleton";
import { ActiveFilters } from "@/components/contracts/ActiveFilters";
import { FilterPanel } from "@/components/contracts/FilterPanel";
import { ResultsCount } from "@/components/contracts/ResultsCount";
import { SearchBar } from "@/components/contracts/SearchBar";
import { SortDropdown } from "@/components/contracts/SortDropdown";
import TagAutocomplete from "@/components/tags/TagAutocomplete";
import {
  Filter,
  Package,
  Search,
  SlidersHorizontal,
  X,
  Sparkles,
  CheckCircle,
  Users,
} from "lucide-react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useAnalytics } from "@/hooks/useAnalytics";
import QueryBuilder from "@/components/contracts/QueryBuilder";
import FavoriteSearches from "@/components/contracts/FavoriteSearches";
import { useFavorites } from "@/hooks/useFavorites";
import {
  DEFAULT_SORT_PREFERENCE,
  persistSortPreference,
  resolveInitialSortPreference,
  type SortBy,
} from "./sort-utils";
import {
  combineAdvancedQueryWithFilters,
  parseAdvancedContractQuery,
} from "@/utils/advancedSearchSyntax";

const DEFAULT_PAGE_SIZE = 12;
const CATEGORY_OPTIONS_NAMES = [
  "DeFi",
  "NFT",
  "Governance",
  "Infrastructure",
  "Payment",
  "Identity",
  "Gaming",
  "Social",
];
const LANGUAGE_OPTIONS = [
  "Rust",
  "TypeScript",
  "JavaScript",
  "AssemblyScript",
  "Move",
];

const ALL_NETWORK_FILTERS = ["mainnet", "testnet", "futurenet"] as const;

function parseCsvOrMulti(values: string[]) {
  return values
    .flatMap((value) => value.split(","))
    .map((value) => value.trim())
    .filter(Boolean);
}

function removeOne<T>(values: T[], value: T) {
  return values.filter((current) => current !== value);
}

function toggleOne<T>(values: T[], value: T) {
  return values.includes(value)
    ? values.filter((current) => current !== value)
    : [...values, value];
}

function getPaginationRange(
  currentPage: number,
  totalPages: number,
): Array<number | "ellipsis"> {
  if (totalPages <= 0) return [];
  if (totalPages <= 5) {
    return Array.from({ length: totalPages }, (_, index) => index + 1);
  }

  const safeCurrentPage = Math.min(Math.max(1, currentPage), totalPages);
  const isNearStart = safeCurrentPage <= 2;
  const isNearEnd = safeCurrentPage >= totalPages - 1;

  const windowStart = isNearStart
    ? 2
    : isNearEnd
      ? totalPages - 2
      : safeCurrentPage - 1;
  const windowEnd = isNearStart
    ? 3
    : isNearEnd
      ? totalPages - 1
      : safeCurrentPage + 1;

  const basePages = [
    1,
    ...Array.from(
      { length: Math.max(0, windowEnd - windowStart + 1) },
      (_, index) => windowStart + index,
    ),
    totalPages,
  ].filter((page) => page >= 1 && page <= totalPages);

  const uniqueSortedPages = Array.from(new Set(basePages)).sort(
    (a, b) => a - b,
  );
  const range: Array<number | "ellipsis"> = [];

  for (let index = 0; index < uniqueSortedPages.length; index += 1) {
    const page = uniqueSortedPages[index];
    const previous = range[range.length - 1];

    if (typeof previous !== "number") {
      range.push(page);
      continue;
    }

    const diff = page - previous;
    if (diff === 2) {
      range.push(previous + 1, page);
      continue;
    }
    if (diff > 2) {
      range.push("ellipsis", page);
      continue;
    }

    range.push(page);
  }

  return range;
}

type FilterOption = {
  value: string;
  label: string;
  count: number;
};

export type ContractsUiFilters = {
  query: string;
  categories: string[];
  languages: string[];
  tags: string[];
  author: string;
  networks: NonNullable<ContractSearchParams["network"]>[];
  verified_only: boolean;
  favorites_only: boolean;
  sort_by: SortBy;
  sort_order: "asc" | "desc";
  page: number;
  page_size: number;
};

type ContractsResponse = Awaited<ReturnType<typeof api.getContracts>>;

const EMPTY_CONTRACTS_RESPONSE: ContractsResponse = {
  items: [],
  total: 0,
  page: 1,
  page_size: DEFAULT_PAGE_SIZE,
  total_pages: 1,
};

const DEFAULT_SORT_BY: SortBy = DEFAULT_SORT_PREFERENCE.sort_by;
const DEFAULT_SORT_ORDER: ContractsUiFilters["sort_order"] =
  DEFAULT_SORT_PREFERENCE.sort_order;
const categoryOptions: FilterOption[] = CATEGORY_OPTIONS_NAMES.map((value) => ({
  value,
  label: value,
  count: 0,
}));
const networkOptions: FilterOption[] = ALL_NETWORK_FILTERS.map((value) => ({
  value,
  label: value.charAt(0).toUpperCase() + value.slice(1),
  count: 0,
}));

export function getInitialFilters(
  searchParams: URLSearchParams,
): ContractsUiFilters {
  const query = searchParams.get("query") || searchParams.get("q") || "";
  const categories = parseCsvOrMulti(searchParams.getAll("category"));
  const languages = parseCsvOrMulti(searchParams.getAll("language"));
  const tags = parseCsvOrMulti(searchParams.getAll("tag"));
  const networks = parseCsvOrMulti(searchParams.getAll("network")).filter(
    (network): network is NonNullable<ContractSearchParams["network"]> =>
      network === "mainnet" || network === "testnet" || network === "futurenet",
  );

  const sortPreference = resolveInitialSortPreference(
    searchParams,
    typeof window !== "undefined" ? window.localStorage : undefined,
  );
  const parsedPage = Number(searchParams.get("page") || "1");

  return {
    query,
    categories,
    languages,
    tags,
    author: searchParams.get("author") || "",
    networks,
    verified_only: searchParams.get("verified_only") === "true",
    favorites_only: searchParams.get("favorites_only") === "true",
    sort_by: sortPreference.sort_by,
    sort_order: sortPreference.sort_order,
    page: Number.isFinite(parsedPage) && parsedPage > 0 ? parsedPage : 1,
    page_size: DEFAULT_PAGE_SIZE,
  };
}

export function buildContractsApiParams(
  filters: ContractsUiFilters,
): ContractSearchParams {
  return {
    query: filters.query || undefined,
    categories: filters.categories.length > 0 ? filters.categories : undefined,
    languages: filters.languages.length > 0 ? filters.languages : undefined,
    tags: filters.tags.length > 0 ? filters.tags : undefined,
    networks:
      filters.networks.length > 0
        ? (filters.networks as Array<"mainnet" | "testnet" | "futurenet">)
        : undefined,
    author: filters.author || undefined,
    verified_only: filters.verified_only || undefined,
    sort_by: filters.sort_by,
    sort_order: filters.sort_order,
    page: filters.page,
    page_size: filters.page_size,
  };
}

function getOptionCounts(
  items: Contract[] | undefined,
  options: readonly string[],
  getValue: (contract: Contract) => string | undefined,
): FilterOption[] {
  const counts = new Map<string, number>();

  items?.forEach((item) => {
    const value = getValue(item);
    if (!value) return;
    counts.set(value, (counts.get(value) ?? 0) + 1);
  });

  return options.map((option) => ({
    value: option,
    label: option.charAt(0).toUpperCase() + option.slice(1),
    count: counts.get(option) ?? 0,
  }));
}

export function ContractsContent() {
  const router = useRouter();
  const pathname = usePathname() ?? "/contracts";
  const searchParams = useSearchParams();
  const { logEvent } = useAnalytics();
  const lastSearchSignatureRef = useRef<string>("");

  const [mobileFiltersOpen, setMobileFiltersOpen] = useState(false);

  const [filters, setFilters] = useState<ContractsUiFilters>(() =>
    getInitialFilters(new URLSearchParams(searchParams?.toString() ?? "")),
  );

  const { favorites } = useFavorites();

  const {
    query,
    categories,
    languages,
    tags,
    networks,
    author,
    verified_only,
    favorites_only,
    sort_by,
    sort_order,
    page,
    page_size,
  } = filters;

  useEffect(() => {
    const params = new URLSearchParams();
    if (query) params.set("query", query);
    categories.forEach((category) => params.append("category", category));
    languages.forEach((language) => params.append("language", language));
    tags.forEach((tag) => params.append("tag", tag));
    networks.forEach((network) => params.append("network", network));
    if (author) params.set("author", author);
    if (verified_only) params.set("verified_only", "true");
    if (favorites_only) params.set("favorites_only", "true");
    if (sort_by !== DEFAULT_SORT_BY || query) params.set("sort_by", sort_by);
    if (sort_order !== DEFAULT_SORT_ORDER) params.set("sort_order", sort_order);
    if (page > 1) params.set("page", String(page));
    if (page_size !== DEFAULT_PAGE_SIZE)
      params.set("page_size", String(page_size));

    const next = params.toString();
    router.replace(next ? `${pathname}?${next}` : pathname, { scroll: false });
  }, [
    query,
    categories,
    languages,
    tags,
    networks,
    author,
    verified_only,
    favorites_only,
    sort_by,
    sort_order,
    page,
    page_size,
    pathname,
    router,
  ]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    persistSortPreference(
      { sort_by: filters.sort_by, sort_order: filters.sort_order },
      window.localStorage,
    );
  }, [filters.sort_by, filters.sort_order]);

  const parsedQuery = useMemo(() => parseAdvancedContractQuery(query), [query]);
  const useAdvancedSearch =
    Boolean(query.trim()) &&
    parsedQuery.usesOr &&
    Boolean(parsedQuery.queryNode);

  const contractsQueryKey = useMemo(
    () => ({
      mode: useAdvancedSearch ? "advanced" : "simple",
      query,
      categories,
      languages,
      tags,
      networks,
      author,
      verified_only,
      favorites_only,
      favoritesList: favorites_only ? favorites : [],
      sort_by,
      sort_order,
      page,
      page_size,
    }),
    [
      author,
      categories,
      languages,
      networks,
      page,
      page_size,
      query,
      sort_by,
      sort_order,
      tags,
      useAdvancedSearch,
      verified_only,
      favorites_only,
      favorites,
    ],
  );

  const {
    data: effectiveData,
    isLoading,
    isFetching,
  } = useQuery<ContractsResponse>({
    queryKey: ["contracts", contractsQueryKey],
    queryFn: async () => {
      if (useAdvancedSearch && parsedQuery.queryNode) {
        const combined = combineAdvancedQueryWithFilters(
          parsedQuery.queryNode,
          {
            categories,
            networks:
              networks.length > 0
                ? (networks as Array<"mainnet" | "testnet" | "futurenet">)
                : undefined,
            tags,
            author,
            verified_only,
            favorites_only: favorites_only || undefined,
            favorites_list: favorites_only ? favorites : undefined,
          },
        );

        return api.advancedSearchContracts({
          query: combined,
          sort_by,
          sort_order,
          limit: page_size,
          offset: (page - 1) * page_size,
        });
      }

      return api.getContracts({
        query,
        categories: categories.length > 0 ? categories : undefined,
        languages: languages.length > 0 ? languages : undefined,
        tags: tags.length > 0 ? tags : undefined,
        networks:
          networks.length > 0
            ? (networks as Array<"mainnet" | "testnet" | "futurenet">)
            : undefined,
        author: author || undefined,
        verified_only: verified_only || undefined,
        favorites_only: favorites_only || undefined,
        favorites_list: favorites_only ? favorites : undefined,
        sort_by,
        sort_order,
        page,
        page_size,
      });
    },
    placeholderData: (previousData) => previousData ?? EMPTY_CONTRACTS_RESPONSE,
  });

  const { data: stats } = useQuery({
    queryKey: ["stats"],
    queryFn: () => api.getStats(),
  });

  // Used to determine if results are empty for UI
  const paginationRange = useMemo(
    () =>
      effectiveData
        ? getPaginationRange(filters.page, effectiveData.total_pages)
        : [],
    [filters.page, effectiveData],
  );

  useEffect(() => {
    const payload = {
      keyword: filters.query || "",
      categories: filters.categories,
      languages: filters.languages,
      networks: filters.networks,
      author: filters.author || undefined,
      verified_only: filters.verified_only,
      sort_by: filters.sort_by,
      page: filters.page,
      page_size: filters.page_size,
    };

    const hasSearchInput =
      Boolean(payload.keyword) ||
      payload.categories.length > 0 ||
      payload.languages.length > 0 ||
      payload.networks.length > 0 ||
      Boolean(payload.author) ||
      payload.verified_only ||
      payload.sort_by !== DEFAULT_SORT_BY ||
      payload.page > 1;

    if (!hasSearchInput) return;

    const signature = JSON.stringify(payload);
    if (lastSearchSignatureRef.current === signature) return;
    lastSearchSignatureRef.current = signature;

    logEvent("search_performed", payload);
  }, [filters.query, filters, logEvent]);

  const clearAllFilters = () =>
    setFilters((current) => ({
      ...current,
      query: "",
      categories: [],
      languages: [],
      tags: [],
      author: "",
      networks: [],
      verified_only: false,
      favorites_only: false,
      sort_by: DEFAULT_SORT_BY,
      sort_order: DEFAULT_SORT_ORDER,
      page: 1,
    }));

  const activeFilterChips = useMemo(() => {
    const chips: Array<{ id: string; label: string; onRemove: () => void }> =
      [];

    if (filters.query) {
      chips.push({
        id: "query",
        label: `Search: ${filters.query}`,
        onRemove: () =>
          setFilters((current) => ({ ...current, query: "", page: 1 })),
      });
    }

    filters.categories.forEach((category) =>
      chips.push({
        id: `category:${category}`,
        label: `Category: ${category}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            categories: removeOne(current.categories, category),
            page: 1,
          })),
      }),
    );

    filters.languages.forEach((language) =>
      chips.push({
        id: `language:${language}`,
        label: `Language: ${language}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            languages: removeOne(current.languages, language),
            page: 1,
          })),
      }),
    );

    filters.tags.forEach((tag) =>
      chips.push({
        id: `tag:${tag}`,
        label: `Tag: ${tag}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            tags: removeOne(current.tags, tag),
            page: 1,
          })),
      }),
    );

    filters.networks.forEach((network) =>
      chips.push({
        id: `network:${network}`,
        label: `Network: ${network}`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            networks: removeOne(current.networks, network),
            page: 1,
          })),
      }),
    );

    if (filters.author) {
      chips.push({
        id: "author",
        label: `Author: ${filters.author}`,
        onRemove: () =>
          setFilters((current) => ({ ...current, author: "", page: 1 })),
      });
    }

    if (filters.verified_only) {
      chips.push({
        id: "verified",
        label: "Verified only",
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            verified_only: false,
            page: 1,
          })),
      });
    }

    if (filters.favorites_only) {
      chips.push({
        id: "favorites",
        label: "Favorites only",
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            favorites_only: false,
            page: 1,
          })),
      });
    }

    if (
      filters.sort_by !== DEFAULT_SORT_BY ||
      filters.sort_order !== DEFAULT_SORT_ORDER
    ) {
      chips.push({
        id: "sort",
        label: `Sort: ${filters.sort_by.replace("_", " ")} (${filters.sort_order})`,
        onRemove: () =>
          setFilters((current) => ({
            ...current,
            sort_by: DEFAULT_SORT_BY,
            sort_order: DEFAULT_SORT_ORDER,
            page: 1,
          })),
      });
    }

    return chips;
  }, [filters]);

  const filterPanelProps = {
    categories: categoryOptions,
    selectedCategories: filters.categories,
    onToggleCategory: (value: string) =>
      setFilters((current) => ({
        ...current,
        categories: toggleOne(current.categories, value),
        page: 1,
      })),
    onClearCategories: () =>
      setFilters((current) => ({
        ...current,
        categories: [],
        page: 1,
      })),
    languages: LANGUAGE_OPTIONS,
    selectedLanguages: filters.languages,
    onToggleLanguage: (value: string) =>
      setFilters((current) => ({
        ...current,
        languages: toggleOne(current.languages, value),
        page: 1,
      })),
    networks: networkOptions,
    selectedNetworks: filters.networks,
    onToggleNetwork: (value: string) =>
      setFilters((current) => ({
        ...current,
        networks: toggleOne(
          current.networks,
          value as ContractsUiFilters["networks"][number],
        ),
        page: 1,
      })),
    onClearNetworks: () =>
      setFilters((current) => ({
        ...current,
        networks: [],
        page: 1,
      })),
    author: filters.author,
    onAuthorChange: (value: string) =>
      setFilters((current) => ({ ...current, author: value, page: 1 })),
    verifiedOnly: filters.verified_only,
    onVerifiedChange: (value: boolean) =>
      setFilters((current) => ({ ...current, verified_only: value, page: 1 })),
    favoritesOnly: filters.favorites_only,
    onFavoritesChange: (value: boolean) =>
      setFilters((current) => ({ ...current, favorites_only: value, page: 1 })),
    activeFilterCount: activeFilterChips.length,
    onResetAll: clearAllFilters,
  };

  return (
    <>
      {/* Hero header with grid pattern */}
      <section className="relative overflow-hidden border-b border-border">
        <div className="absolute inset-0 bg-grid-pattern opacity-5 text-primary" />
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16 relative">
          <div className="text-center max-w-3xl mx-auto">
            <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-primary/10 text-primary text-sm font-medium mb-6">
              <Sparkles className="w-4 h-4" />
              Explore the Soroban Ecosystem
            </div>

            <h1 className="text-4xl sm:text-5xl font-bold mb-4 leading-tight">
              Browse <span className="text-gradient">Contracts</span>
            </h1>
            <p className="text-lg text-muted-foreground mb-10">
              Discover verified Soroban smart contracts on the Stellar network.
              Search, filter, and find the perfect building blocks for your
              project.
            </p>

            {/* Inline search */}
            <div className="max-w-2xl mx-auto mb-10">
              <SearchBar
                value={filters.query}
                onChange={(next: string) =>
                  setFilters((current) => ({ ...current, query: next, page: 1 }))
                }
                onClear={() => setFilters((current) => ({ ...current, query: '', page: 1 }))}
                onCommit={(committed) => {
                  const parsed = parseAdvancedContractQuery(committed);
                  if (parsed.usesOr) {
                    setFilters((current) => ({
                      ...current,
                      query: committed,
                      page: 1,
                    }));
                    return;
                  }

                  setFilters((current) => {
                    const mergedTags = Array.from(
                      new Set([...current.tags, ...parsed.tags]),
                    );
                    return {
                      ...current,
                      query: parsed.cleanedSimpleQuery,
                      tags: mergedTags,
                      page: 1,
                    };
                  });
                }}
                placeholder="Search contracts by name, category, or tag..."
              />
            </div>

            {/* Stats row */}
            <div className="grid grid-cols-3 gap-4 max-w-lg mx-auto">
              <div className="bg-background rounded-xl p-4 border border-border shadow-sm">
                <div className="flex items-center justify-center gap-1.5 mb-1">
                  <Package className="w-4 h-4 text-primary" />
                  <span className="text-2xl font-bold">
                    {stats?.total_contracts ?? "—"}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">Contracts</p>
              </div>
              <div className="bg-background rounded-xl p-4 border border-border shadow-sm">
                <div className="flex items-center justify-center gap-1.5 mb-1">
                  <CheckCircle className="w-4 h-4 text-green-500" />
                  <span className="text-2xl font-bold">
                    {stats?.verified_contracts ?? "—"}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">Verified</p>
              </div>
              <div className="bg-background rounded-xl p-4 border border-border shadow-sm">
                <div className="flex items-center justify-center gap-1.5 mb-1">
                  <Users className="w-4 h-4 text-secondary" />
                  <span className="text-2xl font-bold">
                    {stats?.total_publishers ?? "—"}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground">Publishers</p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Main content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Toolbar */}
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-6">
          <div className="flex items-center gap-3">
            <ResultsCount
              visibleCount={effectiveData?.items.length ?? 0}
              totalCount={effectiveData?.total ?? 0}
            />
            {isFetching && !isLoading && (
              <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                <div className="w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin" />
                Updating...
              </span>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <SortDropdown
              value={filters.sort_by}
              order={filters.sort_order}
              onChange={(value) =>
                setFilters((current) => ({
                  ...current,
                  sort_by: value,
                  page: 1,
                }))
              }
              onOrderChange={(value) =>
                setFilters((current) => ({
                  ...current,
                  sort_order: value,
                  page: 1,
                }))
              }
              showRelevance={!!filters.query}
            />
            <button
              type="button"
              onClick={() => setMobileFiltersOpen(true)}
              className="md:hidden inline-flex items-center gap-2 px-3 py-2 rounded-lg border border-border text-sm text-foreground hover:bg-accent transition-colors"
            >
              <SlidersHorizontal className="w-4 h-4" />
              Filters
            </button>
          </div>
        </div>

        <ActiveFilters chips={activeFilterChips} onClearAll={clearAllFilters} />

        <div className="flex gap-8 mt-6">
          {/* Sidebar filters (desktop) */}
          <aside className="hidden md:flex flex-col w-72 shrink-0 gap-6">
            <div className="gradient-border-card p-5 sticky top-20">
              <div className="flex items-center gap-2 mb-5">
                <Filter className="w-4 h-4 text-primary" />
                <h3 className="text-sm font-semibold text-foreground">
                  Filters
                </h3>
              </div>

              <>
                <FilterPanel {...filterPanelProps} />
                <div className="mt-5 pt-4 border-t border-border">
                  <div className="w-full">
                    <TagAutocomplete
                      onSelect={(tag) =>
                        setFilters((current) => {
                          if (current.tags.includes(tag.name)) return current;
                          return {
                            ...current,
                            tags: [...current.tags, tag.name],
                            page: 1,
                          };
                        })
                      }
                      placeholder="Filter by tag..."
                    />
                  </div>
                </div>
              </>
            </div>
          </aside>

          {/* Results grid */}
          <div className="flex-1 min-w-0">
            {isLoading ? (
              <div
                role="status"
                aria-label="Loading contracts"
                aria-live="polite"
                className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6 mb-8"
              >
                {Array.from({ length: DEFAULT_PAGE_SIZE }).map((_, i) => (
                  <ContractCardSkeleton key={i} />
                ))}
                <span className="sr-only">Loading contracts, please wait…</span>
              </div>
            ) : effectiveData && effectiveData.items.length > 0 ? (
              <>
                <div
                  aria-live="polite"
                  aria-atomic="true"
                  className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6 mb-8 animate-in fade-in duration-300"
                >
                  {effectiveData.items.map((contract: Contract) => (
                    <ContractCard
                      key={contract.id}
                      contract={contract}
                      sortBy={filters.sort_by}
                    />
                  ))}
                </div>

                {effectiveData.total_pages > 1 && (
                  <div className="flex flex-wrap items-center justify-center gap-2 py-4">
                    <button
                      type="button"
                      onClick={() =>
                        setFilters((current) => ({
                          ...current,
                          page: Math.max(1, current.page - 1),
                        }))
                      }
                      disabled={filters.page <= 1}
                      className="px-4 py-2 rounded-lg border border-border text-foreground disabled:opacity-50 disabled:cursor-not-allowed hover:bg-accent transition-colors text-sm font-medium"
                    >
                      Previous
                    </button>

                    {paginationRange.map((item, index) => {
                      if (item === "ellipsis") {
                        return (
                          <span
                            key={`ellipsis-${index}`}
                            className="px-2 text-sm text-muted-foreground"
                            aria-hidden="true"
                          >
                            ...
                          </span>
                        );
                      }

                      const isActive = item === filters.page;

                      return (
                        <button
                          key={item}
                          type="button"
                          onClick={() =>
                            setFilters((current) => ({
                              ...current,
                              page: Math.min(
                                effectiveData.total_pages,
                                Math.max(1, item),
                              ),
                            }))
                          }
                          aria-current={isActive ? "page" : undefined}
                          className={
                            isActive
                              ? "px-3 py-2 rounded-lg bg-primary text-primary-foreground font-medium text-sm btn-glow"
                              : "px-3 py-2 rounded-lg border border-border text-foreground hover:bg-accent transition-colors text-sm"
                          }
                        >
                          {item}
                        </button>
                      );
                    })}

                    <button
                      type="button"
                      onClick={() =>
                        setFilters((current) => ({
                          ...current,
                          page: Math.min(
                            effectiveData.total_pages,
                            current.page + 1,
                          ),
                        }))
                      }
                      disabled={filters.page >= effectiveData.total_pages}
                      className="px-4 py-2 rounded-lg border border-border text-foreground disabled:opacity-50 disabled:cursor-not-allowed hover:bg-accent transition-colors text-sm font-medium"
                    >
                      Next
                    </button>
                  </div>
                )}
              </>
            ) : (
              <div className="text-center py-16 bg-card/50 border border-border rounded-xl">
                <Search className="w-12 h-12 text-muted-foreground mx-auto mb-4 opacity-50" />
                <h3 className="text-lg font-semibold mb-2">
                  No contracts found
                </h3>
                <p className="text-muted-foreground max-w-md mx-auto mb-6 text-sm">
                  We couldn&apos;t find any contracts matching your current filters.
                  Try adjusting your search or clearing some filters.
                </p>
                <button
                  type="button"
                  onClick={clearAllFilters}
                  className="px-4 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 transition-opacity text-sm font-medium"
                >
                  Clear all filters
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {mobileFiltersOpen && (
        <div className="fixed inset-0 z-50 md:hidden">
          <button
            type="button"
            aria-label="Close filters"
            onClick={() => setMobileFiltersOpen(false)}
            className="absolute inset-0 bg-black/50"
          />
          <div className="absolute inset-y-0 right-0 flex w-full max-w-sm flex-col bg-background shadow-2xl">
            <div className="flex items-center justify-between border-b border-border px-4 py-4">
              <div>
                <h2 className="text-base font-semibold text-foreground">
                  Filters
                </h2>
                <p className="text-xs text-muted-foreground">
                  Narrow down contract discovery
                </p>
              </div>
              <button
                type="button"
                onClick={() => setMobileFiltersOpen(false)}
                className="rounded-lg p-2 text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto px-4 py-4">
              <FilterPanel {...filterPanelProps} />

              <div className="mt-5 border-t border-border pt-4">
                <TagAutocomplete
                  onSelect={(tag) =>
                    setFilters((current) => {
                      if (current.tags.includes(tag.name)) return current;
                      return {
                        ...current,
                        tags: [...current.tags, tag.name],
                        page: 1,
                      };
                    })
                  }
                  placeholder="Filter by tag..."
                />
              </div>
            </div>

            <div className="border-t border-border px-4 py-4">
              <button
                type="button"
                onClick={() => setMobileFiltersOpen(false)}
                className="w-full rounded-xl bg-primary px-4 py-3 text-sm font-medium text-primary-foreground"
              >
                Apply filters
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
