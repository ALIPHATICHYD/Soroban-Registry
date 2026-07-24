"use client";

import React, { useState } from "react";
import { X } from "lucide-react";

export interface DiffComment {
  id: string;
  text: string;
  createdAt: string;
}

interface CommentThreadProps {
  lineKey: string;
  comments: DiffComment[];
  onAdd: (lineKey: string, text: string) => void;
  onClose: () => void;
}

export function CommentThread({
  lineKey,
  comments,
  onAdd,
  onClose,
}: CommentThreadProps) {
  const [draft, setDraft] = useState("");

  return (
    <div className="mx-2 mb-2 rounded-xl border border-primary/30 bg-card p-3 text-xs">
      <div className="flex items-center justify-between mb-2">
        <span className="font-semibold text-foreground">Comments</span>
        <button
          type="button"
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground"
        >
          <X size={14} />
        </button>
      </div>

      {comments.length === 0 && (
        <div className="mb-2 text-muted-foreground">No comments yet.</div>
      )}
      {comments.map((c) => (
        <div key={c.id} className="mb-2 rounded-lg bg-accent/30 p-2">
          <div className="text-foreground">{c.text}</div>
          <div className="mt-0.5 text-muted-foreground">
            {new Date(c.createdAt).toLocaleString()}
          </div>
        </div>
      ))}

      <div className="flex gap-2 mt-2">
        <textarea
          className="flex-1 resize-none rounded-lg border border-border bg-background px-2 py-1 text-foreground outline-none focus:border-primary/60"
          rows={2}
          placeholder="Add a comment…"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button
          type="button"
          disabled={!draft.trim()}
          onClick={() => {
            if (draft.trim()) {
              onAdd(lineKey, draft.trim());
              setDraft("");
            }
          }}
          className="self-end rounded-lg bg-primary/10 px-3 py-1 font-semibold text-primary hover:bg-primary/20 disabled:opacity-40"
        >
          Post
        </button>
      </div>
    </div>
  );
}
