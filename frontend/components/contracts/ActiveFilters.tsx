import { X } from "lucide-react";

interface FilterChip {
  id: string;
  label: string;
  onRemove: () => void;
}

interface ActiveFiltersProps {
  chips: FilterChip[];
  onClearAll: () => void;
}

export function ActiveFilters({ chips, onClearAll }: ActiveFiltersProps) {
  if (chips.length === 0) {
    return null;
  }

  return (
    <section className="mt-4 rounded-2xl border border-border bg-card/70 p-4 shadow-sm">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-sm font-semibold text-foreground">
            Active filters
          </p>
          <p className="text-xs text-muted-foreground">
            {chips.length} filter{chips.length === 1 ? "" : "s"} applied
          </p>
        </div>
        <button
          type="button"
          onClick={onClearAll}
          className="text-xs px-3 py-1.5 rounded-full border border-border text-muted-foreground hover:bg-accent"
        >
          Clear all filters
        </button>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {chips.map((chip) => (
          <button
            type="button"
            key={chip.id}
            onClick={chip.onRemove}
            className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs border border-primary/30 bg-primary/10 text-primary"
          >
            {chip.label}
            <X className="w-3 h-3" />
          </button>
        ))}
      </div>
    </section>
  );
}
