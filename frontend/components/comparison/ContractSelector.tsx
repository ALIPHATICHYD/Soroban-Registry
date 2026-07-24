"use client";

import { useMemo, useState } from "react";
import { Search, X, Plus } from "lucide-react";
import type { Contract } from "@/lib/api";

type Props = {
  available: Contract[];
  isLoading: boolean;
  searchQuery: string;
  onSearchQueryChange: (value: string) => void;
  selected: Array<{ id: string; name: string }>;
  onAdd: (contract: Contract) => void;
  onRemove: (contractId: string) => void;
  selectionError?: string | null;
  selectionCountError?: string | null;
};

export default function ContractSelector({
  available,
  isLoading,
  searchQuery,
  onSearchQueryChange,
  selected,
  onAdd,
  onRemove,
  selectionError,
  selectionCountError,
}: Props) {
  const [open, setOpen] = useState(false);

  const selectedIdSet = useMemo(
    () => new Set(selected.map((s) => s.id)),
    [selected],
  );

  const suggestions = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const filtered = q
      ? available.filter(
          (c) =>
            c.name.toLowerCase().includes(q) ||
            c.contract_id.toLowerCase().includes(q),
        )
      : available;
    return filtered.slice(0, 12);
  }, [available, searchQuery]);

  return (
    <section className="rounded-2xl border border-border bg-card p-5">
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">
              Select contracts
            </h2>
            <p className="text-xs text-muted-foreground">
              Pick 2-4 contracts. Duplicates are not allowed.
            </p>
          </div>
        </div>

        <div className="relative">
          <div className="flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 focus-within:ring-2 focus-within:ring-ring">
            <Search className="h-4 w-4 text-muted-foreground" />
            <input
              value={searchQuery}
              onChange={(e) => onSearchQueryChange(e.target.value)}
              onFocus={() => setOpen(true)}
              onBlur={() => setTimeout(() => setOpen(false), 120)}
              placeholder="Search by name or contract ID..."
              className="w-full bg-transparent text-sm text-foreground placeholder:text-muted-foreground outline-none"
            />
          </div>

          {open && (
            <div className="absolute z-20 mt-2 w-full overflow-hidden rounded-xl border border-border bg-card shadow-lg shadow-black/8">
              <div className="max-h-64 overflow-auto">
                {isLoading ? (
                  <div className="px-3 py-3 text-xs text-muted-foreground">
                    Loading contracts...
                  </div>
                ) : suggestions.length === 0 ? (
                  <div className="px-3 py-3 text-xs text-muted-foreground">
                    No matches.
                  </div>
                ) : (
                  <ul className="py-1">
                    {suggestions.map((c) => {
                      const disabled = selectedIdSet.has(c.id);
                      return (
                        <li key={c.id}>
                          <button
                            type="button"
                            disabled={disabled}
                            onMouseDown={(e) => e.preventDefault()}
                            onClick={() => onAdd(c)}
                            className={`flex w-full items-center justify-between gap-3 px-3 py-2 text-left text-sm transition-colors ${
                              disabled
                                ? "cursor-not-allowed text-muted-foreground"
                                : "text-foreground hover:bg-accent"
                            }`}
                          >
                            <div className="min-w-0">
                              <div className="truncate font-medium">
                                {c.name}
                              </div>
                              <div className="truncate font-mono text-xs text-muted-foreground">
                                {c.contract_id}
                              </div>
                            </div>
                            <div className="shrink-0">
                              <span
                                className={`inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs ${
                                  disabled
                                    ? "border-border bg-muted text-muted-foreground"
                                    : "border-primary/20 bg-primary/10 text-primary"
                                }`}
                              >
                                <Plus className="h-3 w-3" />
                                Add
                              </span>
                            </div>
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>
            </div>
          )}
        </div>

        {(selectionError || selectionCountError) && (
          <div className="rounded-xl border border-red-500/20 bg-red-500/5 px-3 py-2 text-xs text-red-600 dark:text-red-400">
            {selectionError || selectionCountError}
          </div>
        )}

        {selected.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {selected.map((s) => (
              <div
                key={s.id}
                className="inline-flex items-center gap-2 rounded-full border border-border bg-accent px-3 py-1 text-xs text-foreground"
              >
                <span className="max-w-[220px] truncate">{s.name}</span>
                <button
                  type="button"
                  onClick={() => onRemove(s.id)}
                  className="rounded-full p-1 text-muted-foreground hover:bg-background hover:text-foreground"
                  aria-label={`Remove ${s.name}`}
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
