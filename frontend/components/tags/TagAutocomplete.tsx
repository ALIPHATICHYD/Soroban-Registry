"use client";

import React, { useState, useEffect, useRef, ChangeEvent } from "react";
import { Tag } from "../../types/tag";
import { getTags } from "../../services/tags.service";
import { Search, Loader2, X } from "lucide-react";

interface TagAutocompleteProps {
  onSelect?: (tag: Tag) => void;
  onClear?: () => void;
  placeholder?: string;
  className?: string;
}

export default function TagAutocomplete({
  onSelect,
  onClear,
  placeholder = "Search tags...",
  className = "",
}: TagAutocompleteProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Tag[]>([]);
  const [loading, setLoading] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);

  const wrapperRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        wrapperRef.current &&
        !wrapperRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(async () => {
      const normalizedQuery = query.trim();

      if (normalizedQuery.length === 0) {
        setResults([]);
        setIsOpen(false);
        setHasSearched(false);
        return;
      }

      setLoading(true);
      setHasSearched(true);
      try {
        const tags = await getTags(normalizedQuery);
        setResults(tags);
        setIsOpen(true);
      } catch {
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [query]);

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
  };

  const handleSelect = (tag: Tag) => {
    setQuery(tag.name);
    setIsOpen(false);
    if (onSelect) {
      onSelect(tag);
    }
  };

  const clearSearch = () => {
    setQuery("");
    setResults([]);
    setIsOpen(false);
    setHasSearched(false);
    if (onClear) onClear();
    inputRef.current?.focus();
  };

  // Helper to highlight matching text
  const highlightMatch = (text: string, highlight: string) => {
    if (!highlight.trim()) {
      return <span>{text}</span>;
    }
    const regex = new RegExp(
      `(${highlight.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`,
      "gi",
    );
    const parts = text.split(regex);
    return (
      <span>
        {parts.map((part, i) =>
          regex.test(part) ? (
            <span
              key={i}
              className="font-bold text-primary bg-primary/10 rounded-sm px-0.5"
            >
              {part}
            </span>
          ) : (
            <span key={i}>{part}</span>
          ),
        )}
      </span>
    );
  };

  return (
    <div ref={wrapperRef} className={`relative w-full max-w-md ${className}`}>
      <div className="relative">
        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
          <Search className="h-4 w-4 text-muted-foreground" />
        </div>
        <input
          ref={inputRef}
          type="text"
          className="block w-full pl-10 pr-10 py-2 border border-border rounded-lg leading-5 bg-card placeholder-muted-foreground focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary sm:text-sm transition duration-150 ease-in-out"
          placeholder={placeholder}
          value={query}
          onChange={handleInputChange}
          onFocus={() => {
            if (results.length > 0 && query.trim().length > 0) setIsOpen(true);
          }}
          aria-expanded={isOpen}
          aria-autocomplete="list"
          role="combobox"
          aria-controls="tag-results"
        />
        <div className="absolute inset-y-0 right-0 pr-3 flex items-center">
          {loading ? (
            <Loader2 className="h-4 w-4 text-muted-foreground animate-spin" />
          ) : query ? (
            <X
              className="h-4 w-4 text-muted-foreground hover:text-foreground cursor-pointer"
              onClick={clearSearch}
            />
          ) : null}
        </div>
      </div>

      {isOpen && (
        <ul
          id="tag-results"
          className="absolute z-10 mt-1 w-full bg-card shadow-lg max-h-60 rounded-lg py-1 text-base ring-1 ring-border overflow-auto focus:outline-none sm:text-sm"
          role="listbox"
        >
          {results.length > 0
            ? results.map((tag) => (
                <li
                  key={tag.id}
                  className="cursor-pointer select-none relative py-2 pl-3 pr-4 hover:bg-accent text-foreground group"
                  role="option"
                  aria-selected={false}
                  onClick={() => handleSelect(tag)}
                >
                  <div className="flex justify-between items-center">
                    <span className="block truncate">
                      {highlightMatch(tag.name, query)}
                    </span>
                    <span className="text-xs text-muted-foreground group-hover:text-foreground">
                      ({tag.usageCount} uses)
                    </span>
                  </div>
                </li>
              ))
            : hasSearched &&
              !loading && (
                <li className="cursor-default select-none relative py-2 pl-3 pr-9 text-muted-foreground italic">
                  No matching tags
                </li>
              )}
        </ul>
      )}
    </div>
  );
}
