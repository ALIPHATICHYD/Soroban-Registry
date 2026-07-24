"use client";

import { useState, useRef, useEffect } from "react";
import { Folder, FolderPlus, Pencil, Trash2, X, Check } from "lucide-react";
import { useCollections } from "@/hooks/useCollections";
import type { Collection } from "@/store/slices/collectionsSlice";

interface CollectionsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectCollection: (id: string | null) => void;
  activeCollectionId: string | null;
}

function CollectionRow({
  collection,
  isActive,
  onSelect,
  onDelete,
  onRename,
}: {
  collection: Collection;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
  onRename: (name: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(collection.name);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const commitRename = () => {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== collection.name) onRename(trimmed);
    setEditing(false);
  };

  return (
    <div
      className={`group flex items-center gap-2 rounded-lg px-3 py-2 transition-colors cursor-pointer ${
        isActive
          ? "bg-primary/10 border border-primary/20"
          : "hover:bg-accent border border-transparent"
      }`}
      onClick={!editing ? onSelect : undefined}
    >
      <Folder
        className={`w-4 h-4 shrink-0 ${isActive ? "text-primary" : "text-muted-foreground"}`}
      />

      {editing ? (
        <input
          ref={inputRef}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitRename();
            if (e.key === "Escape") { setDraft(collection.name); setEditing(false); }
          }}
          onBlur={commitRename}
          onClick={(e) => e.stopPropagation()}
          className="flex-1 min-w-0 bg-transparent text-sm text-foreground outline-none border-b border-primary"
        />
      ) : (
        <span className="flex-1 min-w-0 truncate text-sm text-foreground">
          {collection.name}
        </span>
      )}

      <span className="text-xs text-muted-foreground shrink-0">
        {collection.items.length}
      </span>

      <div
        className="hidden group-hover:flex items-center gap-1 shrink-0"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          type="button"
          onClick={() => { setDraft(collection.name); setEditing(true); }}
          className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors"
          aria-label="Rename collection"
        >
          <Pencil className="w-3.5 h-3.5" />
        </button>
        <button
          type="button"
          onClick={onDelete}
          className="p-1 rounded hover:bg-red-500/10 text-muted-foreground hover:text-red-500 transition-colors"
          aria-label="Delete collection"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      </div>

      {isActive && !editing && (
        <Check className="w-3.5 h-3.5 text-primary shrink-0" />
      )}
    </div>
  );
}

export default function CollectionsModal({
  isOpen,
  onClose,
  onSelectCollection,
  activeCollectionId,
}: CollectionsModalProps) {
  const { collections, createCollection, deleteCollection, renameCollection } =
    useCollections();
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (creating) inputRef.current?.focus();
  }, [creating]);

  const handleCreate = () => {
    const name = newName.trim();
    if (!name) return;
    createCollection(name);
    setNewName("");
    setCreating(false);
  };

  const handleDelete = (id: string) => {
    if (activeCollectionId === id) onSelectCollection(null);
    deleteCollection(id);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-end p-4 pt-20">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="relative w-full max-w-xs bg-card border border-border rounded-2xl shadow-2xl flex flex-col max-h-[70vh] animate-modal-in">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold text-foreground">Collections</h2>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Collection list */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {collections.length === 0 && !creating && (
            <p className="text-xs text-muted-foreground text-center py-6">
              No collections yet. Create one below.
            </p>
          )}

          {collections.map((col) => (
            <CollectionRow
              key={col.id}
              collection={col}
              isActive={activeCollectionId === col.id}
              onSelect={() => {
                onSelectCollection(
                  activeCollectionId === col.id ? null : col.id,
                );
                onClose();
              }}
              onDelete={() => handleDelete(col.id)}
              onRename={(name) => renameCollection(col.id, name)}
            />
          ))}

          {/* Inline create form */}
          {creating && (
            <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-primary/30 bg-primary/5">
              <Folder className="w-4 h-4 text-primary shrink-0" />
              <input
                ref={inputRef}
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleCreate();
                  if (e.key === "Escape") {
                    setCreating(false);
                    setNewName("");
                  }
                }}
                placeholder="Collection name…"
                className="flex-1 min-w-0 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
              />
              <button
                type="button"
                onClick={handleCreate}
                disabled={!newName.trim()}
                className="p-1 rounded text-primary hover:bg-primary/10 disabled:opacity-40 transition-colors"
              >
                <Check className="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onClick={() => { setCreating(false); setNewName(""); }}
                className="p-1 rounded text-muted-foreground hover:bg-accent transition-colors"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          )}
        </div>

        {/* Footer */}
        {!creating && (
          <div className="px-2 py-2 border-t border-border">
            <button
              type="button"
              onClick={() => setCreating(true)}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
            >
              <FolderPlus className="w-4 h-4" />
              New collection
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
