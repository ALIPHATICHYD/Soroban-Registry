"use client";

import { useQueries } from "@tanstack/react-query";
import {
  Star,
  BookmarkX,
  ArrowRight,
  Share2,
  FolderPlus,
  Folder,
  ChevronRight,
} from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { api } from "@/lib/api";
import ContractCard from "@/components/ContractCard";
import ContractCardSkeleton from "@/components/ContractCardSkeleton";
import Navbar from "@/components/Navbar";
import { useFavorites } from "@/hooks/useFavorites";
import { useCopy } from "@/hooks/useCopy";
import { useCollections } from "@/hooks/useCollections";
import CollectionsModal from "@/components/favorites/CollectionsModal";
import AddToCollectionMenu from "@/components/favorites/AddToCollectionMenu";

export default function FavoritesPage() {
  const {
    favorites,
    isLoading: favoritesLoading,
    clearAllFavorites,
  } = useFavorites();
  const { collections, removeFromCollection } = useCollections();
  const [confirmingClear, setConfirmingClear] = useState(false);
  const [collectionsOpen, setCollectionsOpen] = useState(false);
  const [activeCollectionId, setActiveCollectionId] = useState<string | null>(
    null,
  );
  const { copy, copied } = useCopy();

  const activeCollection = activeCollectionId
    ? collections.find((c) => c.id === activeCollectionId) ?? null
    : null;

  // IDs to display: active collection's items, or all favorites
  const displayIds = activeCollection ? activeCollection.items : favorites;

  // Fetch contract data for each displayed ID in parallel
  const contractQueries = useQueries({
    queries: displayIds.map((id) => ({
      queryKey: ["contract", id],
      queryFn: () => api.getContract(id),
      retry: false,
    })),
  });

  const isLoading =
    favoritesLoading || contractQueries.some((q) => q.isLoading);

  const loadedContracts = contractQueries
    .filter((q) => q.isSuccess && q.data)
    .map((q) => q.data!);

  const handleClearAll = () => {
    if (confirmingClear) {
      clearAllFavorites();
      setConfirmingClear(false);
      setActiveCollectionId(null);
    } else {
      setConfirmingClear(true);
    }
  };

  const handleCancelClear = () => setConfirmingClear(false);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        {/* Hero / heading */}
        <div className="mb-6 flex items-start justify-between gap-4 flex-wrap">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-xl bg-yellow-500/10 border border-yellow-500/20 flex items-center justify-center">
                <Star className="w-5 h-5 text-yellow-500" fill="currentColor" />
              </div>
              <div className="flex items-center gap-2">
                <h1 className="text-3xl font-bold text-foreground">
                  {activeCollection ? (
                    <>
                      <button
                        type="button"
                        onClick={() => setActiveCollectionId(null)}
                        className="text-muted-foreground hover:text-foreground transition-colors"
                      >
                        Your Favorites
                      </button>
                      <ChevronRight className="inline w-5 h-5 mx-1 text-muted-foreground" />
                      {activeCollection.name}
                    </>
                  ) : (
                    "Your Favorites"
                  )}
                  {!isLoading && (
                    <span className="ml-2 text-xl font-normal text-muted-foreground">
                      ({loadedContracts.length})
                    </span>
                  )}
                </h1>
              </div>
            </div>
            <p className="text-muted-foreground">
              {activeCollection
                ? `Viewing collection — ${activeCollection.items.length} contract${activeCollection.items.length !== 1 ? "s" : ""}`
                : "Contracts you've saved for quick access."}
            </p>
          </div>

          {favorites.length > 0 && !isLoading && (
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                onClick={() => setCollectionsOpen(true)}
                className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                <FolderPlus className="w-4 h-4" />
                Collections
                {collections.length > 0 && (
                  <span className="ml-0.5 rounded-full bg-primary/10 text-primary px-1.5 py-0.5 text-[10px] font-semibold">
                    {collections.length}
                  </span>
                )}
              </button>
              <button
                type="button"
                onClick={() => {
                  if (typeof window !== "undefined") {
                    const url = new URL(window.location.href);
                    url.searchParams.set("list", favorites.join(","));
                    copy(url.toString(), {
                      successMessage: "Favorites list link copied!",
                      failureMessage: "Failed to copy link",
                    });
                  }
                }}
                className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                <Share2 className="w-4 h-4" />
                {copied ? "Copied Link" : "Share"}
              </button>
              <div className="w-px h-6 bg-border mx-1 hidden sm:block" />
              {confirmingClear ? (
                <>
                  <span className="text-sm text-muted-foreground">
                    Remove all {favorites.length} favorites?
                  </span>
                  <button
                    type="button"
                    onClick={handleClearAll}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-sm font-medium text-red-500 transition-colors hover:bg-red-500/20"
                  >
                    <BookmarkX className="w-4 h-4" />
                    Confirm
                  </button>
                  <button
                    type="button"
                    onClick={handleCancelClear}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-foreground transition-colors hover:bg-accent"
                  >
                    Cancel
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  onClick={handleClearAll}
                  className="inline-flex items-center gap-1.5 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                >
                  <BookmarkX className="w-4 h-4" />
                  Remove all favorites
                </button>
              )}
            </div>
          )}
        </div>

        {/* Collection filter tabs */}
        {collections.length > 0 && !isLoading && favorites.length > 0 && (
          <div className="mb-8 flex items-center gap-2 flex-wrap">
            <button
              type="button"
              onClick={() => setActiveCollectionId(null)}
              className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-sm font-medium transition-colors ${
                !activeCollectionId
                  ? "bg-primary text-primary-foreground"
                  : "bg-accent text-muted-foreground hover:text-foreground"
              }`}
            >
              All favorites
            </button>
            {collections.map((col) => (
              <button
                key={col.id}
                type="button"
                onClick={() =>
                  setActiveCollectionId(
                    activeCollectionId === col.id ? null : col.id,
                  )
                }
                className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-sm font-medium transition-colors ${
                  activeCollectionId === col.id
                    ? "bg-primary text-primary-foreground"
                    : "bg-accent text-muted-foreground hover:text-foreground"
                }`}
              >
                <Folder className="w-3.5 h-3.5" />
                {col.name}
                <span className="opacity-70">{col.items.length}</span>
              </button>
            ))}
          </div>
        )}

        {/* Loading state */}
        {isLoading && (
          <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
            {Array.from({ length: Math.max(displayIds.length, 3) }).map(
              (_, i) => (
                <ContractCardSkeleton key={i} />
              ),
            )}
          </div>
        )}

        {/* Empty state — all favorites */}
        {!isLoading && favorites.length === 0 && (
          <div className="flex flex-col items-center justify-center py-24 text-center">
            <div className="w-16 h-16 rounded-2xl bg-yellow-500/10 border border-yellow-500/20 flex items-center justify-center mb-6">
              <Star className="w-8 h-8 text-yellow-500/50" />
            </div>
            <h2 className="text-xl font-semibold text-foreground mb-2">
              No favorites saved yet
            </h2>
            <p className="text-muted-foreground max-w-sm mb-8">
              Browse contracts and click the <strong>Save</strong> button on any
              card to add it here.
            </p>
            <Link
              href="/contracts"
              className="inline-flex items-center gap-2 rounded-xl bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-all hover:brightness-110 btn-glow"
            >
              Browse contracts
              <ArrowRight className="w-4 h-4" />
            </Link>
          </div>
        )}

        {/* Empty state — collection is empty */}
        {!isLoading &&
          activeCollection &&
          activeCollection.items.length === 0 && (
            <div className="flex flex-col items-center justify-center py-24 text-center">
              <div className="w-16 h-16 rounded-2xl bg-primary/10 border border-primary/20 flex items-center justify-center mb-6">
                <Folder className="w-8 h-8 text-primary/50" />
              </div>
              <h2 className="text-xl font-semibold text-foreground mb-2">
                {activeCollection.name} is empty
              </h2>
              <p className="text-muted-foreground max-w-sm mb-8">
                Use the <strong>Collect</strong> button on any saved contract to
                add it here.
              </p>
              <button
                type="button"
                onClick={() => setActiveCollectionId(null)}
                className="inline-flex items-center gap-2 rounded-xl bg-primary px-5 py-2.5 text-sm font-semibold text-primary-foreground transition-all hover:brightness-110 btn-glow"
              >
                Back to all favorites
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          )}

        {/* Contracts grid */}
        {!isLoading && loadedContracts.length > 0 && (
          <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
            {loadedContracts.map((contract) => (
              <div key={contract.id} className="flex flex-col gap-1">
                <ContractCard contract={contract} />
                <div className="flex items-center gap-2 px-1">
                  <AddToCollectionMenu contractId={contract.id} />
                  {activeCollectionId && (
                    <button
                      type="button"
                      onClick={() =>
                        removeFromCollection(activeCollectionId, contract.id)
                      }
                      className="inline-flex items-center gap-1 rounded-md border border-border bg-card px-2.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-500"
                    >
                      <BookmarkX className="h-3.5 w-3.5" />
                      Remove from collection
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </main>

      <CollectionsModal
        isOpen={collectionsOpen}
        onClose={() => setCollectionsOpen(false)}
        onSelectCollection={setActiveCollectionId}
        activeCollectionId={activeCollectionId}
      />
    </div>
  );
}
