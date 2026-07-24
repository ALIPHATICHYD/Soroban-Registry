import {
  ArrowDown,
  ArrowUp,
  Check,
  Clock3,
  Flame,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import type { SortBy, SortOrder } from "@/app/contracts/sort-utils";

interface SortDropdownProps {
  value: SortBy;
  order: SortOrder;
  onChange: (value: SortBy) => void;
  onOrderChange: (value: SortOrder) => void;
  showRelevance?: boolean;
}

type SortOption = {
  value: SortBy;
  label: string;
  icon: typeof Clock3;
};

const BASE_SORT_OPTIONS: SortOption[] = [
  { value: "created_at", label: "Newest", icon: Clock3 },
  { value: "updated_at", label: "Last Updated", icon: RefreshCw },
  { value: "popularity", label: "Most Popular", icon: Flame },
];

export function SortDropdown({
  value,
  order,
  onChange,
  onOrderChange,
  showRelevance,
}: SortDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [focusedIndex, setFocusedIndex] = useState(0);
  const menuRef = useRef<HTMLDivElement | null>(null);

  const options = useMemo(() => {
    const extended = [...BASE_SORT_OPTIONS];
    if (showRelevance) {
      extended.unshift({
        value: "relevance",
        label: "Relevance",
        icon: Sparkles,
      });
    }
    return extended;
  }, [showRelevance]);

  const selected =
    options.find((option) => option.value === value) ?? options[0];

  useEffect(() => {
    if (!isOpen) return;
    const listener = (event: MouseEvent) => {
      if (!menuRef.current) return;
      if (!menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    window.addEventListener("mousedown", listener);
    return () => window.removeEventListener("mousedown", listener);
  }, [isOpen]);

  useEffect(() => {
    const index = options.findIndex((option) => option.value === value);
    queueMicrotask(() => setFocusedIndex(index >= 0 ? index : 0));
  }, [options, value]);

  const toggleOrder = () => {
    onOrderChange(order === "asc" ? "desc" : "asc");
  };

  return (
    <div ref={menuRef} className="relative flex items-center gap-2">
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        className="inline-flex min-w-44 items-center justify-between gap-2 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-primary/40 focus:outline-none focus:ring-2 focus:ring-primary/20"
        onClick={() => setIsOpen((open) => !open)}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setIsOpen(true);
            setFocusedIndex((index) => Math.min(index + 1, options.length - 1));
          }
          if (event.key === "ArrowUp") {
            event.preventDefault();
            setIsOpen(true);
            setFocusedIndex((index) => Math.max(index - 1, 0));
          }
          if (event.key === "Enter" && isOpen) {
            event.preventDefault();
            onChange(options[focusedIndex].value);
            setIsOpen(false);
          }
          if (event.key === "Escape") {
            setIsOpen(false);
          }
        }}
      >
        <span className="inline-flex items-center gap-2">
          <selected.icon className="h-4 w-4 text-primary" />
          {selected.label}
        </span>
        <span className="text-xs text-muted-foreground">Sort</span>
      </button>

      <button
        type="button"
        aria-label={order === "asc" ? "Sort ascending" : "Sort descending"}
        onClick={toggleOrder}
        className="inline-flex items-center gap-1 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-primary/40 focus:outline-none focus:ring-2 focus:ring-primary/20"
      >
        {order === "asc" ? (
          <ArrowUp className="h-4 w-4" />
        ) : (
          <ArrowDown className="h-4 w-4" />
        )}
        {order === "asc" ? "Asc" : "Desc"}
      </button>

      {isOpen && (
        <div
          role="listbox"
          aria-label="Sort contracts"
          className="absolute right-0 top-12 z-40 w-64 rounded-xl border border-border bg-card p-1 shadow-lg"
        >
          {options.map((option, index) => {
            const Icon = option.icon;
            const active = option.value === value;
            return (
              <button
                key={option.value}
                type="button"
                role="option"
                aria-selected={active}
                className={`flex w-full items-center justify-between rounded-lg px-3 py-2 text-sm transition-colors ${active ? "bg-primary/10 text-primary" : "text-foreground hover:bg-accent"} ${focusedIndex === index ? "outline-none ring-1 ring-primary/40" : ""}`}
                onMouseEnter={() => setFocusedIndex(index)}
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                }}
              >
                <span className="inline-flex items-center gap-2">
                  <Icon className="h-4 w-4" />
                  {option.label}
                </span>
                {active ? <Check className="h-4 w-4" /> : null}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
