"use client";

import React, { useEffect, useMemo, useRef, useState } from "react";
import { Search, X } from "lucide-react";
import type { SearchSuggestion } from "@/types";
import { api } from "@/lib/api";

interface SearchBarProps {
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
  onCommit?: (value: string) => void;
  placeholder?: string;
}

type MenuItem = {
  text: string;
  kind: string;
  source: "recent" | "suggestion";
  score: number;
};

const RECENT_SEARCH_KEY = "contract-search-recent";
const MAX_RECENT_SEARCHES = 5;
const SEARCH_HINTS = [
  "Search by contract name, category, creator, or tag.",
  'Try "DeFi", "NFT", "token", or a publisher address.',
  "Advanced: use tag:yield and OR (e.g. token OR bridge).",
  "Use the keyboard arrows to navigate suggestions.",
];

const SUGGESTION_LABELS: Record<string, string> = {
  contract: "Name",
  category: "Category",
  publisher: "Creator",
  creator: "Creator",
  tag: "Tag",
  recent: "Recent search",
  default: "Suggestion",
};

function loadRecentSearches(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(RECENT_SEARCH_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (Array.isArray(parsed)) {
      return parsed
        .filter((value) => typeof value === "string" && value.trim())
        .slice(0, MAX_RECENT_SEARCHES);
    }
  } catch {
    // ignore malformed local storage data
  }

  return [];
}

function saveRecentSearch(query: string): string[] {
  if (typeof window === "undefined") return [];
  const trimmed = query.trim();
  if (!trimmed) return [];

  const existing = loadRecentSearches();
  const deduped = [
    trimmed,
    ...existing.filter((item) => item.toLowerCase() !== trimmed.toLowerCase()),
  ];
  const next = deduped.slice(0, MAX_RECENT_SEARCHES);

  try {
    window.localStorage.setItem(RECENT_SEARCH_KEY, JSON.stringify(next));
  } catch {
    // ignore storage failures
  }

  return next;
}

function highlightMatch(text: string, query: string) {
  if (!query) return text;
  const escaped = query.replace(/[.*+?^${}()|[\\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "i");
  const lowerQuery = query.toLowerCase();

  return text.split(regex).map((fragment, index) =>
    fragment.toLowerCase() === lowerQuery ? (
      <span key={index} className="font-semibold text-foreground">
        {fragment}
      </span>
    ) : (
      <span key={index}>{fragment}</span>
    ),
  );
}

export function SearchBar({
  value,
  onChange,
  onClear,
  onCommit,
  placeholder = "Search contracts by name, category, or tag...",
}: SearchBarProps) {
  const [suggestions, setSuggestions] = useState<SearchSuggestion[]>([]);
  const [recentSearches, setRecentSearches] = useState<string[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const [isLoading, setIsLoading] = useState(false);
  const [hasError, setHasError] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const latestQueryRef = useRef(value);

  useEffect(() => {
    requestAnimationFrame(() => setRecentSearches(loadRecentSearches()));
  }, []);

  useEffect(() => {
    latestQueryRef.current = value;

    if (!value.trim()) {
      requestAnimationFrame(() => {
        setSuggestions([]);
        setIsLoading(false);
        setHasError(false);
        setIsOpen(recentSearches.length > 0);
        setHighlightedIndex(-1);
      });
      return;
    }

    requestAnimationFrame(() => {
      setIsLoading(true);
      setHasError(false);
    });

    const delay = window.setTimeout(async () => {
      try {
        const result = await api.getContractSearchSuggestions(value, 8);
        if (latestQueryRef.current !== value) return;

        const sorted = [...result.items].sort((a, b) => {
          if (b.score !== a.score) return b.score - a.score;
          return a.text.localeCompare(b.text);
        });

        setSuggestions(sorted);
        setIsOpen(true);
        setHighlightedIndex(-1);
      } catch {
        if (latestQueryRef.current === value) {
          setHasError(true);
          setSuggestions([]);
          setIsOpen(true);
        }
      } finally {
        if (latestQueryRef.current === value) {
          setIsLoading(false);
        }
      }
    }, 220);

    return () => window.clearTimeout(delay);
  }, [value, recentSearches.length]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
        setHighlightedIndex(-1);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const menuItems = useMemo<MenuItem[]>(() => {
    if (value.trim()) {
      return suggestions.map((suggestion) => ({
        text: suggestion.text,
        kind: suggestion.kind,
        source: "suggestion",
        score: suggestion.score,
      }));
    }

    return recentSearches.map((text) => ({
      text,
      kind: "recent",
      source: "recent",
      score: 1,
    }));
  }, [recentSearches, suggestions, value]);

  const commitSearch = (text: string) => {
    onChange(text);
    setIsOpen(false);
    setHighlightedIndex(-1);
    if (text.trim()) {
      setRecentSearches(saveRecentSearch(text));
      onCommit?.(text);
    }
  };

  const onInputKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (menuItems.length === 0) return;
      setIsOpen(true);
      setHighlightedIndex((current) =>
        current < menuItems.length - 1 ? current + 1 : 0,
      );
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (menuItems.length === 0) return;
      setIsOpen(true);
      setHighlightedIndex((current) =>
        current > 0 ? current - 1 : menuItems.length - 1,
      );
      return;
    }

    if (event.key === "Enter") {
      if (isOpen && highlightedIndex >= 0 && menuItems[highlightedIndex]) {
        event.preventDefault();
        commitSearch(menuItems[highlightedIndex].text);
        return;
      }
      if (value.trim()) {
        commitSearch(value);
      }
      return;
    }

    if (event.key === "Escape") {
      setIsOpen(false);
      setHighlightedIndex(-1);
    }
  };

  const hintText = value.trim()
    ? "Matching terms are highlighted and suggestions update as you type."
    : "Type a contract name, category, creator, or tag to get instant results.";

  return (
    <div ref={containerRef} className="relative">
      <div className="relative">
        <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-muted-foreground" />
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            setIsOpen(true);
            setHighlightedIndex(-1);
          }}
          onFocus={() => {
            if (value.trim() || recentSearches.length > 0) {
              setIsOpen(true);
            }
          }}
          onKeyDown={onInputKeyDown}
          placeholder={placeholder}
          aria-label="Search contracts"
          aria-keyshortcuts="/"
          aria-autocomplete="list"
          aria-expanded={isOpen}
          aria-activedescendant={
            highlightedIndex >= 0
              ? `search-suggestion-${highlightedIndex}`
              : undefined
          }
          className="w-full pl-12 pr-12 py-4 rounded-xl border border-border bg-background text-foreground placeholder-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary shadow-lg"
        />
        {value && (
          <button
            type="button"
            onClick={() => {
              onClear();
              setIsOpen(false);
              setHighlightedIndex(-1);
            }}
            className="absolute right-3 top-1/2 -translate-y-1/2 p-1 rounded-md text-muted-foreground hover:text-foreground transition-colors"
            aria-label="Clear search"
          >
            <X className="w-4 h-4" />
          </button>
        )}
      </div>

      <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
        {SEARCH_HINTS.map((hint) => (
          <span
            key={hint}
            className="rounded-full border border-border bg-card px-3 py-1"
          >
            {hint}
          </span>
        ))}
      </div>
      <p className="mt-2 text-xs text-muted-foreground">{hintText}</p>

      {isOpen && (
        <div className="absolute z-20 w-full mt-3 overflow-hidden rounded-3xl border border-border bg-card shadow-2xl">
          {isLoading ? (
            <div className="px-4 py-4 flex items-center justify-between gap-3 text-sm text-muted-foreground">
              <span>Loading suggestions...</span>
            </div>
          ) : menuItems.length > 0 ? (
            <ul role="listbox" className="divide-y divide-border">
              {menuItems.map((item, index) => (
                <li
                  key={`${item.source}-${item.text}-${index}`}
                  id={`search-suggestion-${index}`}
                  role="option"
                  aria-selected={highlightedIndex === index}
                  onMouseDown={(event) => {
                    event.preventDefault();
                    commitSearch(item.text);
                  }}
                  onMouseEnter={() => setHighlightedIndex(index)}
                  className={`cursor-pointer px-4 py-3 hover:bg-primary/10 transition-colors ${
                    highlightedIndex === index ? "bg-primary/10" : ""
                  }`}
                >
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-foreground">
                      {highlightMatch(item.text, value.trim())}
                    </span>
                    <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase tracking-[0.12em] text-muted-foreground">
                      {SUGGESTION_LABELS[item.kind] ??
                        SUGGESTION_LABELS.default}
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <div className="px-4 py-4 text-sm text-muted-foreground">
              {hasError
                ? "Unable to load suggestions. Try again or press Enter to search."
                : "No suggestions found. Press Enter to search with your current query."}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
