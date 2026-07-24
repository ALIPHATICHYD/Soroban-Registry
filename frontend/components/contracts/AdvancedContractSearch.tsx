"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import {
  Bookmark,
  ChevronDown,
  Loader2,
  Search,
  SlidersHorizontal,
  X,
} from "lucide-react";
import { api } from "@/lib/api";
import type { Contract, ContractSearchParams } from "@/types";
import ContractCard from "@/components/ContractCard";
import ContractCardSkeleton from "@/components/ContractCardSkeleton";

// ── Types ──────────────────────────────────────────────────────────────────────

type Network = "mainnet" | "testnet" | "futurenet";

interface AdvancedSearchFilters {
  query: string;
  networks: Network[];
  categories: string[];
  verified_only: boolean;
  page: number;
}

interface SavedSearch {
  id: string;
  label: string;
  filters: AdvancedSearchFilters;
  savedAt: number;
}

// ── Constants ──────────────────────────────────────────────────────────────────

const NETWORK_OPTIONS: { value: Network; label: string }[] = [
  { value: "mainnet", label: "Mainnet" },
  { value: "testnet", label: "Testnet" },
  { value: "futurenet", label: "Futurenet" },
];

const CATEGORY_OPTIONS = [
  "DeFi",
  "NFT",
  "Governance",
  "Infrastructure",
  "Payment",
  "Identity",
  "Gaming",
  "Social",
];

const SAVED_SEARCHES_KEY = "soroban_registry_saved_searches";
const PAGE_SIZE = 12;
const SEARCH_DEBOUNCE_MS = 250;

const EMPTY_STATE_SUGGESTIONS = [
  "defi",
  "nft",
  "token",
  "governance",
  "payment",
];

const DEFAULT_FILTERS: AdvancedSearchFilters = {
  query: "",
  networks: [],
  categories: [],
  verified_only: false,
  page: 1,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

function filtersToParams(filters: AdvancedSearchFilters): ContractSearchParams {
  return {
    query: filters.query || undefined,
    networks: filters.networks.length > 0 ? filters.networks : undefined,
    categories: filters.categories.length > 0 ? filters.categories : undefined,
    verified_only: filters.verified_only || undefined,
    page: filters.page,
    page_size: PAGE_SIZE,
    sort_by: "relevance",
  };
}

function filtersToUrlParams(filters: AdvancedSearchFilters): URLSearchParams {
  const params = new URLSearchParams();
  if (filters.query) params.set("q", filters.query);
  if (filters.networks.length > 0)
    params.set("networks", filters.networks.join(","));
  if (filters.categories.length > 0)
    params.set("categories", filters.categories.join(","));
  if (filters.verified_only) params.set("verified", "1");
  if (filters.page > 1) params.set("page", String(filters.page));
  return params;
}

function urlParamsToFilters(params: URLSearchParams): AdvancedSearchFilters {
  const networksRaw = params.get("networks") ?? "";
  const categoriesRaw = params.get("categories") ?? "";
  return {
    query: params.get("q") ?? "",
    networks: networksRaw
      ? (networksRaw.split(",").filter(Boolean) as Network[])
      : [],
    categories: categoriesRaw ? categoriesRaw.split(",").filter(Boolean) : [],
    verified_only: params.get("verified") === "1",
    page: Math.max(1, Number(params.get("page") ?? 1)),
  };
}

function hasActiveFilters(filters: AdvancedSearchFilters): boolean {
  return (
    !!filters.query ||
    filters.networks.length > 0 ||
    filters.categories.length > 0 ||
    filters.verified_only
  );
}

function loadSavedSearches(): SavedSearch[] {
  if (typeof window === "undefined") return [];
  try {
    return JSON.parse(localStorage.getItem(SAVED_SEARCHES_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function persistSavedSearches(searches: SavedSearch[]) {
  localStorage.setItem(
    SAVED_SEARCHES_KEY,
    JSON.stringify(searches.slice(0, 10)),
  );
}

function toggle<T>(arr: T[], item: T): T[] {
  return arr.includes(item) ? arr.filter((x) => x !== item) : [...arr, item];
}

// ── MultiSelectDropdown ────────────────────────────────────────────────────────

interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectDropdownProps {
  id: string;
  label: string;
  options: MultiSelectOption[];
  selected: string[];
  onToggle: (value: string) => void;
  onClear: () => void;
}

function MultiSelectDropdown({
  id,
  label,
  options,
  selected,
  onToggle,
  onClear,
}: MultiSelectDropdownProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const summary =
    selected.length === 0
      ? `All ${label}`
      : selected.length <= 2
        ? options
            .filter((o) => selected.includes(o.value))
            .map((o) => o.label)
            .join(", ")
        : `${selected.length} selected`;

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        id={id}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className={`flex items-center gap-2 px-3 py-2 rounded-lg border text-sm transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40 ${
          selected.length > 0
            ? "border-primary bg-primary/10 text-primary font-medium"
            : "border-border bg-card text-foreground hover:border-primary/40"
        }`}
      >
        <span className="whitespace-nowrap">{summary}</span>
        {selected.length > 0 && (
          <span
            className="flex h-4 w-4 items-center justify-center rounded-full bg-primary text-[10px] text-white font-bold"
            aria-hidden="true"
          >
            {selected.length}
          </span>
        )}
        <ChevronDown
          className={`h-3.5 w-3.5 shrink-0 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <div
          role="listbox"
          aria-multiselectable="true"
          aria-label={label}
          className="absolute left-0 top-full z-20 mt-1 min-w-[160px] rounded-xl border border-border bg-card shadow-lg ring-1 ring-black/5"
        >
          <ul className="max-h-56 overflow-y-auto py-1">
            {options.map((opt) => {
              const checked = selected.includes(opt.value);
              return (
                <li key={opt.value}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={checked}
                    onClick={() => onToggle(opt.value)}
                    className="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-left hover:bg-accent transition-colors"
                  >
                    <span
                      className={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                        checked
                          ? "border-primary bg-primary text-white"
                          : "border-border bg-card"
                      }`}
                      aria-hidden="true"
                    >
                      {checked && (
                        <svg
                          viewBox="0 0 10 8"
                          className="h-2.5 w-2.5"
                          fill="currentColor"
                        >
                          <path
                            d="M1 4l3 3 5-6"
                            stroke="currentColor"
                            strokeWidth="1.5"
                            fill="none"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                          />
                        </svg>
                      )}
                    </span>
                    {opt.label}
                  </button>
                </li>
              );
            })}
          </ul>
          {selected.length > 0 && (
            <div className="border-t border-border px-3 py-1.5">
              <button
                type="button"
                onClick={onClear}
                className="text-xs text-muted-foreground hover:text-foreground transition-colors"
              >
                Clear selection
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── FilterChip ─────────────────────────────────────────────────────────────────

interface FilterChipProps {
  label: string;
  onRemove: () => void;
}

function FilterChip({ label, onRemove }: FilterChipProps) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-primary/30 bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary">
      {label}
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${label} filter`}
        className="rounded-full hover:bg-primary/20 p-0.5 transition-colors focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <X className="h-2.5 w-2.5" />
      </button>
    </span>
  );
}

// ── EmptyState ─────────────────────────────────────────────────────────────────

interface EmptyStateProps {
  query: string;
  onSuggestion: (term: string) => void;
  onClearFilters: () => void;
}

function EmptyState({ query, onSuggestion, onClearFilters }: EmptyStateProps) {
  return (
    <div
      role="status"
      aria-live="polite"
      className="flex flex-col items-center justify-center py-16 text-center"
    >
      <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-accent">
        <Search className="h-7 w-7 text-muted-foreground" />
      </div>
      <h3 className="mb-2 text-lg font-semibold text-foreground">
        {query ? `No results for "${query}"` : "No contracts found"}
      </h3>
      <p className="mb-6 max-w-xs text-sm text-muted-foreground">
        Try a different search term, remove some filters, or explore these
        popular categories:
      </p>
      <div className="mb-6 flex flex-wrap justify-center gap-2">
        {EMPTY_STATE_SUGGESTIONS.map((term) => (
          <button
            key={term}
            type="button"
            onClick={() => onSuggestion(term)}
            className="rounded-full border border-border bg-card px-3 py-1.5 text-sm text-foreground hover:border-primary/40 hover:bg-accent transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40"
          >
            {term}
          </button>
        ))}
      </div>
      <button
        type="button"
        onClick={onClearFilters}
        className="text-sm font-medium text-primary hover:underline focus:outline-none"
      >
        Clear all filters
      </button>
    </div>
  );
}

// ── SavedSearchesPanel ─────────────────────────────────────────────────────────

interface SavedSearchesPanelProps {
  searches: SavedSearch[];
  onLoad: (filters: AdvancedSearchFilters) => void;
  onDelete: (id: string) => void;
  onClose: () => void;
}

function SavedSearchesPanel({
  searches,
  onLoad,
  onDelete,
  onClose,
}: SavedSearchesPanelProps) {
  return (
    <div className="absolute right-0 top-full z-30 mt-1 w-72 rounded-xl border border-border bg-card shadow-lg ring-1 ring-black/5">
      <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
        <span className="text-sm font-semibold text-foreground">
          Saved searches
        </span>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close saved searches"
          className="rounded p-0.5 hover:bg-accent transition-colors"
        >
          <X className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
      </div>
      {searches.length === 0 ? (
        <p className="px-4 py-6 text-center text-sm text-muted-foreground">
          No saved searches yet.
        </p>
      ) : (
        <ul className="max-h-64 overflow-y-auto py-1">
          {searches.map((s) => (
            <li
              key={s.id}
              className="flex items-center gap-2 px-3 py-2 hover:bg-accent"
            >
              <button
                type="button"
                onClick={() => {
                  onLoad(s.filters);
                  onClose();
                }}
                className="flex-1 text-left text-sm text-foreground truncate focus:outline-none focus:underline"
              >
                {s.label}
              </button>
              <button
                type="button"
                onClick={() => onDelete(s.id)}
                aria-label={`Delete saved search "${s.label}"`}
                className="shrink-0 rounded p-0.5 hover:bg-red-50 hover:text-red-500 transition-colors"
              >
                <X className="h-3 w-3" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

// ── AdvancedContractSearch ─────────────────────────────────────────────────────

export function AdvancedContractSearch() {
  const router = useRouter();
  const searchParams = useSearchParams();

  // Initialise from URL on mount only.
  const [filters, setFilters] = useState<AdvancedSearchFilters>(() =>
    urlParamsToFilters(new URLSearchParams(searchParams?.toString() ?? "")),
  );

  // Debounced query sent to API — prevents firing on every keystroke.
  const [debouncedQuery, setDebouncedQuery] = useState(filters.query);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [showMobileFilters, setShowMobileFilters] = useState(false);
  const [showSavedSearches, setShowSavedSearches] = useState(false);
  const [savedSearches, setSavedSearches] =
    useState<SavedSearch[]>(loadSavedSearches);
  const savedSearchesButtonRef = useRef<HTMLButtonElement>(null);
  const savedSearchesPanelRef = useRef<HTMLDivElement>(null);

  // Close saved-searches panel when clicking outside.
  useEffect(() => {
    if (!showSavedSearches) return;
    const handler = (e: MouseEvent) => {
      if (
        !savedSearchesPanelRef.current?.contains(e.target as Node) &&
        !savedSearchesButtonRef.current?.contains(e.target as Node)
      ) {
        setShowSavedSearches(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showSavedSearches]);

  // Sync filters → URL (replaceState to avoid polluting history on every keystroke).
  useEffect(() => {
    const params = filtersToUrlParams(filters);
    const newUrl = params.toString()
      ? `?${params.toString()}`
      : window.location.pathname;
    router.replace(newUrl, { scroll: false });
  }, [filters, router]);

  // Debounce query changes.
  const handleQueryChange = useCallback((value: string) => {
    setFilters((prev) => ({ ...prev, query: value, page: 1 }));
    if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    debounceTimerRef.current = setTimeout(() => {
      setDebouncedQuery(value);
    }, SEARCH_DEBOUNCE_MS);
  }, []);

  // For non-query filter changes we update debounced immediately.
  const updateFilters = useCallback((patch: Partial<AdvancedSearchFilters>) => {
    setFilters((prev) => ({ ...prev, ...patch, page: 1 }));
    setDebouncedQuery((prev) =>
      "query" in patch ? (patch.query ?? "") : prev,
    );
  }, []);

  const clearAllFilters = useCallback(() => {
    setFilters(DEFAULT_FILTERS);
    setDebouncedQuery("");
  }, []);

  // Build effective search params (uses debouncedQuery for text).
  const apiParams = useMemo<ContractSearchParams>(
    () =>
      filtersToParams({
        ...filters,
        query: debouncedQuery,
      }),
    [filters, debouncedQuery],
  );

  const { data, isFetching, isPending } = useQuery({
    queryKey: ["contracts", apiParams],
    queryFn: () => api.getContracts(apiParams),
    placeholderData: (prev) => prev,
    staleTime: 30_000,
  });

  const contracts: Contract[] = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = data?.total_pages ?? 0;

  // Build filter chips.
  const filterChips = useMemo(() => {
    const chips: { id: string; label: string; onRemove: () => void }[] = [];
    filters.networks.forEach((n) =>
      chips.push({
        id: `network-${n}`,
        label: n.charAt(0).toUpperCase() + n.slice(1),
        onRemove: () =>
          updateFilters({ networks: toggle(filters.networks, n) }),
      }),
    );
    filters.categories.forEach((c) =>
      chips.push({
        id: `category-${c}`,
        label: c,
        onRemove: () =>
          updateFilters({ categories: toggle(filters.categories, c) }),
      }),
    );
    if (filters.verified_only) {
      chips.push({
        id: "verified",
        label: "Verified only",
        onRemove: () => updateFilters({ verified_only: false }),
      });
    }
    return chips;
  }, [filters, updateFilters]);

  // Save current search.
  function handleSaveSearch() {
    if (!hasActiveFilters(filters)) return;
    const parts: string[] = [];
    if (filters.query) parts.push(`"${filters.query}"`);
    filters.categories.forEach((c) => parts.push(c));
    filters.networks.forEach((n) => parts.push(n));
    if (filters.verified_only) parts.push("verified");

    const newSearch: SavedSearch = {
      id: Date.now().toString(),
      label: parts.join(" · ") || "Search",
      filters,
      savedAt: Date.now(),
    };
    const updated = [
      newSearch,
      ...savedSearches.filter((s) => s.label !== newSearch.label),
    ];
    setSavedSearches(updated);
    persistSavedSearches(updated);
  }

  function handleDeleteSavedSearch(id: string) {
    const updated = savedSearches.filter((s) => s.id !== id);
    setSavedSearches(updated);
    persistSavedSearches(updated);
  }

  const isLoading = isPending && !data;

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {/* ── Header ── */}
        <header className="mb-6">
          <h1 className="text-2xl font-bold text-foreground">
            Contract Search
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Search and filter Soroban smart contracts by network, category, and
            more.
          </p>
        </header>

        {/* ── Search input row ── */}
        <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-center">
          <label htmlFor="contract-search" className="sr-only">
            Search contracts
          </label>
          <div className="relative flex-1">
            <Search
              className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden="true"
            />
            {isFetching && (
              <Loader2
                className="absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 animate-spin text-primary"
                aria-hidden="true"
              />
            )}
            <input
              id="contract-search"
              type="search"
              role="searchbox"
              aria-label="Search contracts"
              placeholder="Search by name, keyword, or contract ID…"
              value={filters.query}
              onChange={(e) => handleQueryChange(e.target.value)}
              className="w-full rounded-xl border border-border bg-card py-2.5 pl-10 pr-10 text-sm text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/30 transition-colors"
            />
          </div>

          {/* Mobile filter toggle */}
          <button
            type="button"
            onClick={() => setShowMobileFilters((v) => !v)}
            aria-expanded={showMobileFilters}
            aria-controls="filter-controls"
            className="flex items-center gap-2 rounded-xl border border-border bg-card px-4 py-2.5 text-sm font-medium text-foreground hover:bg-accent transition-colors sm:hidden focus:outline-none focus:ring-2 focus:ring-primary/40"
          >
            <SlidersHorizontal className="h-4 w-4" />
            Filters
            {filterChips.length > 0 && (
              <span className="flex h-5 w-5 items-center justify-center rounded-full bg-primary text-[10px] text-white font-bold">
                {filterChips.length}
              </span>
            )}
          </button>
        </div>

        {/* ── Filter controls (desktop: always visible, mobile: toggle) ── */}
        <div
          id="filter-controls"
          role="group"
          aria-label="Search filters"
          className={`mb-4 flex flex-wrap items-center gap-2 ${showMobileFilters ? "flex" : "hidden sm:flex"}`}
        >
          <MultiSelectDropdown
            id="network-filter"
            label="Network"
            options={NETWORK_OPTIONS}
            selected={filters.networks}
            onToggle={(v) =>
              updateFilters({
                networks: toggle(filters.networks, v as Network),
              })
            }
            onClear={() => updateFilters({ networks: [] })}
          />

          <MultiSelectDropdown
            id="category-filter"
            label="Category"
            options={CATEGORY_OPTIONS.map((c) => ({ value: c, label: c }))}
            selected={filters.categories}
            onToggle={(v) =>
              updateFilters({ categories: toggle(filters.categories, v) })
            }
            onClear={() => updateFilters({ categories: [] })}
          />

          <label className="flex cursor-pointer items-center gap-2 rounded-lg border border-border bg-card px-3 py-2 text-sm transition-colors hover:border-primary/40">
            <input
              type="checkbox"
              checked={filters.verified_only}
              onChange={(e) =>
                updateFilters({ verified_only: e.target.checked })
              }
              className="h-3.5 w-3.5 accent-primary"
              aria-label="Show verified contracts only"
            />
            <span className="text-foreground">Verified only</span>
          </label>

          {/* Spacer */}
          <div className="ml-auto flex items-center gap-2">
            {/* Save Search */}
            <div className="relative" ref={savedSearchesPanelRef}>
              <button
                ref={savedSearchesButtonRef}
                type="button"
                onClick={() => setShowSavedSearches((v) => !v)}
                aria-label="Saved searches"
                aria-expanded={showSavedSearches}
                className="flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:border-primary/40 transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40"
              >
                <Bookmark className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">Saved</span>
                {savedSearches.length > 0 && (
                  <span className="flex h-4 w-4 items-center justify-center rounded-full bg-secondary text-[10px] text-white font-bold">
                    {savedSearches.length}
                  </span>
                )}
              </button>

              {showSavedSearches && (
                <SavedSearchesPanel
                  searches={savedSearches}
                  onLoad={(f) => {
                    setFilters(f);
                    setDebouncedQuery(f.query);
                  }}
                  onDelete={handleDeleteSavedSearch}
                  onClose={() => setShowSavedSearches(false)}
                />
              )}
            </div>

            {hasActiveFilters(filters) && (
              <button
                type="button"
                onClick={handleSaveSearch}
                className="flex items-center gap-1.5 rounded-lg border border-primary/30 bg-primary/10 px-3 py-2 text-sm font-medium text-primary hover:bg-primary/20 transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40"
              >
                <Bookmark className="h-3.5 w-3.5" />
                Save search
              </button>
            )}
          </div>
        </div>

        {/* ── Active filter chips ── */}
        {filterChips.length > 0 && (
          <section
            aria-label="Active filters"
            className="mb-4 flex flex-wrap items-center gap-2"
          >
            {filterChips.map((chip) => (
              <FilterChip
                key={chip.id}
                label={chip.label}
                onRemove={chip.onRemove}
              />
            ))}
            <button
              type="button"
              onClick={clearAllFilters}
              className="ml-1 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors focus:outline-none focus:underline"
            >
              Clear all
            </button>
          </section>
        )}

        {/* ── Results count ── */}
        {!isLoading && (
          <p
            aria-live="polite"
            aria-atomic="true"
            className="mb-4 text-sm text-muted-foreground"
          >
            {total === 0
              ? "No contracts found"
              : `Showing ${contracts.length} of ${total.toLocaleString()} contract${total === 1 ? "" : "s"}`}
          </p>
        )}

        {/* ── Results grid ── */}
        {isLoading ? (
          <div
            aria-busy="true"
            aria-label="Loading contracts"
            className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
          >
            {Array.from({ length: PAGE_SIZE }).map((_, i) => (
              <ContractCardSkeleton key={i} />
            ))}
          </div>
        ) : contracts.length === 0 ? (
          <EmptyState
            query={debouncedQuery}
            onSuggestion={(term) => {
              clearAllFilters();
              handleQueryChange(term);
            }}
            onClearFilters={clearAllFilters}
          />
        ) : (
          <div
            className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
            aria-label="Contract search results"
          >
            {contracts.map((contract) => (
              <ContractCard key={contract.id} contract={contract} />
            ))}
          </div>
        )}

        {/* ── Pagination ── */}
        {totalPages > 1 && (
          <nav
            aria-label="Search results pagination"
            className="mt-8 flex items-center justify-center gap-1"
          >
            <button
              type="button"
              disabled={filters.page <= 1}
              onClick={() => setFilters((f) => ({ ...f, page: f.page - 1 }))}
              aria-label="Previous page"
              className="rounded-lg border border-border bg-card px-3 py-2 text-sm text-foreground hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40"
            >
              ‹ Prev
            </button>

            <span className="px-4 text-sm text-muted-foreground">
              Page {filters.page} of {totalPages}
            </span>

            <button
              type="button"
              disabled={filters.page >= totalPages}
              onClick={() => setFilters((f) => ({ ...f, page: f.page + 1 }))}
              aria-label="Next page"
              className="rounded-lg border border-border bg-card px-3 py-2 text-sm text-foreground hover:bg-accent disabled:opacity-40 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-primary/40"
            >
              Next ›
            </button>
          </nav>
        )}
      </div>
    </div>
  );
}

export default AdvancedContractSearch;
