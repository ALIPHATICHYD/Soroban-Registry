"use client";

import { useState, useRef, useEffect } from "react";
import { FolderPlus, Check, Folder, FolderMinus } from "lucide-react";
import { useCollections } from "@/hooks/useCollections";

interface AddToCollectionMenuProps {
  contractId: string;
}

export default function AddToCollectionMenu({
  contractId,
}: AddToCollectionMenuProps) {
  const { collections, addToCollection, removeFromCollection } =
    useCollections();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  if (collections.length === 0) return null;

  const memberOf = collections
    .filter((c) => c.items.includes(contractId))
    .map((c) => c.id);

  const toggle = (
    e: React.MouseEvent,
    collectionId: string,
    isMember: boolean,
  ) => {
    e.preventDefault();
    e.stopPropagation();
    if (isMember) removeFromCollection(collectionId, contractId);
    else addToCollection(collectionId, contractId);
  };

  return (
    <div ref={ref} className="relative" onClick={(e) => e.preventDefault()}>
      <button
        type="button"
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setOpen((v) => !v);
        }}
        className={`inline-flex items-center gap-1 rounded-md border px-2.5 py-1 text-xs font-medium transition-colors duration-150 ${
          memberOf.length > 0
            ? "border-primary/30 bg-primary/10 text-primary hover:bg-primary/20"
            : "border-border bg-card text-foreground hover:bg-accent"
        }`}
        aria-label="Add to collection"
        title="Add to collection"
      >
        {memberOf.length > 0 ? (
          <Folder className="h-3.5 w-3.5" fill="currentColor" />
        ) : (
          <FolderPlus className="h-3.5 w-3.5" />
        )}
        <span>
          {memberOf.length > 0 ? `In ${memberOf.length}` : "Collect"}
        </span>
      </button>

      {open && (
        <div className="absolute bottom-full mb-1 right-0 z-50 min-w-[160px] rounded-xl border border-border bg-card shadow-xl py-1">
          {collections.map((col) => {
            const isMember = col.items.includes(contractId);
            return (
              <button
                key={col.id}
                type="button"
                onClick={(e) => toggle(e, col.id, isMember)}
                className="w-full flex items-center gap-2 px-3 py-2 text-xs hover:bg-accent transition-colors text-left"
              >
                {isMember ? (
                  <FolderMinus className="w-3.5 h-3.5 text-primary shrink-0" />
                ) : (
                  <Folder className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                )}
                <span className="flex-1 truncate text-foreground">
                  {col.name}
                </span>
                {isMember && (
                  <Check className="w-3 h-3 text-primary shrink-0" />
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
