import React, { useState, useMemo, useRef, useEffect } from "react";
import { ContractSummary } from "@/types/publisher";
import { Search, Filter, ArrowUpRight } from "lucide-react";
import { VerificationBadge } from "./VerificationBadge";
import Link from "next/link";

interface PublisherContractsListProps {
  contracts: ContractSummary[];
}

type FilterStatus = "all" | "verified" | "failed" | "pending";

export function PublisherContractsList({
  contracts,
}: PublisherContractsListProps) {
  const [statusFilter, setStatusFilter] = useState<FilterStatus>("all");
  const [searchTerm, setSearchTerm] = useState("");
  const searchInputRef = useRef<HTMLInputElement>(null);

  const filteredContracts = useMemo(() => {
    return contracts
      .filter((contract) => {
        const matchesStatus =
          statusFilter === "all" ||
          contract.verificationStatus === statusFilter;
        const matchesSearch =
          contract.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
          contract.description.toLowerCase().includes(searchTerm.toLowerCase());
        return matchesStatus && matchesSearch;
      })
      .sort(
        (a, b) =>
          new Date(b.deployedAt).getTime() - new Date(a.deployedAt).getTime(),
      );
  }, [contracts, statusFilter, searchTerm]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const isSlashShortcut = event.key === "/" || event.code === "Slash";
      if (!isSlashShortcut || event.ctrlKey || event.metaKey || event.altKey)
        return;

      const activeElement = document.activeElement as HTMLElement | null;
      const isTypingField = Boolean(
        activeElement &&
        (activeElement.tagName === "INPUT" ||
          activeElement.tagName === "TEXTAREA" ||
          activeElement.tagName === "SELECT" ||
          activeElement.isContentEditable),
      );

      if (isTypingField) return;

      event.preventDefault();
      searchInputRef.current?.focus();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <div className="bg-card rounded-2xl shadow-sm border border-border p-6">
      <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 mb-6">
        <h2 className="text-xl font-bold text-foreground flex items-center gap-2">
          Published Contracts
          <span className="text-sm font-normal text-muted-foreground bg-accent px-2 py-0.5 rounded-full">
            {filteredContracts.length}
          </span>
        </h2>

        <div className="flex flex-col sm:flex-row gap-3 w-full md:w-auto">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              ref={searchInputRef}
              type="text"
              placeholder="Search contracts..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              aria-label="Search contracts"
              aria-keyshortcuts="/"
              className="pl-9 pr-4 py-2 w-full sm:w-64 bg-accent border border-border rounded-lg text-sm focus:ring-2 focus:ring-primary/40 focus:border-transparent outline-none transition-all"
            />
          </div>

          <div className="relative">
            <Filter className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as FilterStatus)}
              className="pl-9 pr-8 py-2 w-full sm:w-40 bg-accent border border-border rounded-lg text-sm appearance-none focus:ring-2 focus:ring-primary/40 focus:border-transparent outline-none cursor-pointer"
            >
              <option value="all">All Status</option>
              <option value="verified">Verified</option>
              <option value="pending">Pending</option>
              <option value="failed">Failed</option>
            </select>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
        {filteredContracts.length > 0 ? (
          filteredContracts.map((contract) => (
            <Link
              key={contract.id}
              href={`/contracts/${contract.id}`}
              className="group block bg-accent border border-border rounded-lg p-5 hover:border-primary transition-colors"
            >
              <div className="flex justify-between items-start mb-3">
                <VerificationBadge status={contract.verificationStatus} />
                <ArrowUpRight className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>

              <h3 className="font-semibold text-foreground mb-2 group-hover:text-primary transition-colors truncate">
                {contract.name}
              </h3>

              <p className="text-sm text-muted-foreground line-clamp-2 mb-4 h-10">
                {contract.description}
              </p>

              {contract.tags && contract.tags.length > 0 && (
                <div className="flex flex-wrap gap-2 mb-4">
                  {contract.tags.slice(0, 3).map((tag) => (
                    <span
                      key={tag}
                      className="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-muted text-muted-foreground"
                    >
                      {tag}
                    </span>
                  ))}
                  {contract.tags.length > 3 && (
                    <span className="text-xs text-muted-foreground flex items-center">
                      +{contract.tags.length - 3}
                    </span>
                  )}
                </div>
              )}

              <div className="flex items-center justify-between text-xs text-muted-foreground pt-3 border-t border-border">
                <span>ID: {contract.id.substring(0, 8)}...</span>
                <span>
                  {new Date(contract.deployedAt).toLocaleDateString()}
                </span>
              </div>
            </Link>
          ))
        ) : (
          <div className="col-span-full py-12 text-center text-muted-foreground bg-accent rounded-lg border border-dashed border-border">
            <p>No contracts found matching your filters.</p>
          </div>
        )}
      </div>
    </div>
  );
}
