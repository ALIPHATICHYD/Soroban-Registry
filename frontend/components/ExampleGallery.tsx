"use client";

import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import ExampleCard from "./ExampleCard";
import ExampleCardSkeleton from "./ExampleCardSkeleton";
import { AlertCircle, Terminal, Search } from "lucide-react";
import { useAnalytics } from "@/hooks/useAnalytics";

interface ExampleGalleryProps {
  contractId: string;
}

export default function ExampleGallery({ contractId }: ExampleGalleryProps) {
  const {
    data: examples,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["contract-examples", contractId],
    queryFn: () => api.getContractExamples(contractId),
  });
  const { logEvent } = useAnalytics();

  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    if (!error) return;
    logEvent("error_event", {
      source: "example_gallery",
      contract_id: contractId,
      message: "Failed to load examples",
    });
  }, [error, contractId, logEvent]);

  if (isLoading) {
    return (
      <div className="space-y-8">
        <div className="flex flex-col gap-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-2xl font-bold text-foreground">
                Usage Examples
              </h2>
            </div>
          </div>
        </div>
        {/* Skeleton: 1 col mobile → 2 col tablet → 3 col desktop */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          <ExampleCardSkeleton />
          <ExampleCardSkeleton />
          <ExampleCardSkeleton />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 bg-red-500/10 text-red-500 rounded-xl flex items-center gap-2">
        <AlertCircle className="w-5 h-5" />
        Failed to load examples
      </div>
    );
  }

  if (!examples || examples.length === 0) {
    return (
      <div className="text-center py-12 bg-accent rounded-xl border border-dashed border-border">
        <Terminal className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
        <h3 className="text-lg font-medium text-foreground mb-2">
          No Examples Yet
        </h3>
        <p className="text-muted-foreground max-w-sm mx-auto">
          There are no code examples for this contract yet. Be the first to
          contribute one!
        </p>
      </div>
    );
  }

  const filteredExamples = examples.filter((e) => {
    const matchesCategory =
      selectedCategory === "all" || e.category === selectedCategory;
    const matchesSearch =
      !searchQuery.trim() ||
      e.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      e.description?.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const categories = ["all", "basic", "advanced", "integration"];

  return (
    <div className="space-y-8">
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-2xl font-bold text-foreground">
              Usage Examples
            </h2>
            <span className="px-2 py-1 rounded-full bg-accent text-xs font-medium text-muted-foreground">
              {filteredExamples.length}
            </span>
          </div>
        </div>

        {/* Search + category filters: stack on mobile, row on sm+ */}
        <div className="flex flex-col sm:flex-row gap-4 justify-between">
          <div className="relative w-full sm:max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search examples by title or description..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-9 pr-4 py-2 rounded-lg border border-border bg-card text-sm focus:ring-2 focus:ring-primary/40 outline-none transition-all min-h-[44px]"
            />
          </div>

          {/* Category tabs: scroll horizontally on mobile if needed */}
          <div className="flex p-1 bg-accent rounded-lg overflow-x-auto shrink-0">
            {categories.map((cat) => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-4 py-2 rounded-md text-sm font-medium transition-all capitalize whitespace-nowrap min-h-[44px] ${
                  selectedCategory === cat
                    ? "bg-card shadow-sm text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {cat}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/*
        Responsive grid:
        - mobile (default): 1 card per row
        - sm / tablet 640px+: 2 cards per row
        - lg / desktop 1024px+: 3 cards per row
      */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
        {filteredExamples.length > 0 ? (
          filteredExamples.map((example) => (
            <ExampleCard key={example.id} example={example} />
          ))
        ) : (
          // Empty state spans all columns so it stays centered
          <div className="col-span-full text-center py-12 bg-accent rounded-xl">
            <p className="text-muted-foreground">
              No examples found matching your criteria.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
