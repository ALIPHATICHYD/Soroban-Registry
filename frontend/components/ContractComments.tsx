"use client";

import { useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { Comment } from "@/lib/api";
import { formatPublicKey } from "@/lib/utils/formatting";
import {
  ChevronUp,
  ChevronDown,
  MessageSquare,
  Flag,
  AlertTriangle,
} from "lucide-react";

interface ContractCommentsProps {
  contractId: string;
}

// Minimal inline markdown renderer: handles bold, italic, inline code, fenced code blocks.
function renderMarkdown(text: string): React.ReactNode[] {
  const lines = text.split("\n");
  const result: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    if (line.trimStart().startsWith("```")) {
      const fence = line.trimStart().startsWith("```") ? "```" : "~~~";
      const lang = line.trimStart().slice(fence.length).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trimStart().startsWith(fence)) {
        codeLines.push(lines[i]);
        i++;
      }
      result.push(
        <pre
          key={i}
          className="my-2 overflow-x-auto rounded-lg border border-border bg-background p-3 text-xs font-mono leading-5 text-foreground"
          data-lang={lang || undefined}
        >
          {codeLines.join("\n")}
        </pre>,
      );
      i++;
      continue;
    }

    result.push(
      <span key={i} className="block leading-6">
        {renderInline(line)}
      </span>,
    );
    i++;
  }

  return result;
}

function renderInline(text: string): React.ReactNode[] {
  // Tokenise: **bold**, _italic_, `code`
  const pattern = /(\*\*[\s\S]+?\*\*|_[\s\S]+?_|`[^`]+`)/g;
  const parts: React.ReactNode[] = [];
  let last = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > last) {
      parts.push(text.slice(last, match.index));
    }
    const token = match[0];
    if (token.startsWith("**")) {
      parts.push(<strong key={match.index}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith("_")) {
      parts.push(<em key={match.index}>{token.slice(1, -1)}</em>);
    } else {
      parts.push(
        <code
          key={match.index}
          className="rounded bg-accent px-1 py-0.5 font-mono text-xs text-foreground"
        >
          {token.slice(1, -1)}
        </code>,
      );
    }
    last = match.index + token.length;
  }

  if (last < text.length) {
    parts.push(text.slice(last));
  }

  return parts;
}

function formatRelative(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function truncateAddress(addr: string): string {
  return formatPublicKey(addr);
}

// ──────────────────────────────────────────────────────────────────────────────

interface CommentCardProps {
  comment: Comment;
  contractId: string;
  isReply?: boolean;
}

function CommentCard({
  comment,
  contractId,
  isReply = false,
}: CommentCardProps) {
  const queryClient = useQueryClient();
  const [replyOpen, setReplyOpen] = useState(false);
  const [replyBody, setReplyBody] = useState("");

  const voteMutation = useMutation({
    mutationFn: (direction: "up" | "down") =>
      api.voteComment(comment.id, contractId, direction),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["contract-comments", contractId],
      });
    },
  });

  const flagMutation = useMutation({
    mutationFn: () => api.flagComment(comment.id, contractId, "spam"),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["contract-comments", contractId],
      });
    },
  });

  const replyMutation = useMutation({
    mutationFn: (body: string) => api.postComment(contractId, body, comment.id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["contract-comments", contractId],
      });
      setReplyOpen(false);
      setReplyBody("");
    },
  });

  if (comment.flagged) {
    return (
      <div
        className={`rounded-xl border border-border bg-muted/40 p-4 ${isReply ? "ml-8" : ""}`}
      >
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <AlertTriangle className="w-3.5 h-3.5" />
          This comment has been flagged for review.
        </div>
      </div>
    );
  }

  return (
    <div
      className={`rounded-xl border border-border bg-card p-4 ${isReply ? "ml-8" : ""}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xs font-mono font-medium text-foreground">
            {truncateAddress(comment.author)}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatRelative(comment.created_at)}
          </span>
        </div>
      </div>

      {/* Body */}
      <div className="text-sm text-foreground mb-3 leading-relaxed">
        {renderMarkdown(comment.body)}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-4">
        {/* Vote */}
        <div className="flex items-center gap-1">
          <button
            type="button"
            aria-label="Upvote"
            onClick={() => voteMutation.mutate("up")}
            disabled={voteMutation.isPending}
            className="p-0.5 rounded text-muted-foreground hover:text-primary transition-colors disabled:opacity-50"
          >
            <ChevronUp className="w-4 h-4" />
          </button>
          <span className="text-xs font-semibold tabular-nums text-foreground w-5 text-center">
            {comment.score}
          </span>
          <button
            type="button"
            aria-label="Downvote"
            onClick={() => voteMutation.mutate("down")}
            disabled={voteMutation.isPending}
            className="p-0.5 rounded text-muted-foreground hover:text-red-500 transition-colors disabled:opacity-50"
          >
            <ChevronDown className="w-4 h-4" />
          </button>
        </div>

        {/* Reply (only on top-level) */}
        {!isReply && (
          <button
            type="button"
            onClick={() => setReplyOpen((v) => !v)}
            className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            <MessageSquare className="w-3.5 h-3.5" />
            Reply
          </button>
        )}

        {/* Flag */}
        <button
          type="button"
          aria-label="Flag as spam or abuse"
          onClick={() => flagMutation.mutate()}
          disabled={flagMutation.isPending || comment.flag_count > 0}
          className="ml-auto flex items-center gap-1 text-xs text-muted-foreground hover:text-red-500 transition-colors disabled:opacity-40"
        >
          <Flag className="w-3 h-3" />
          Flag
        </button>
      </div>

      {/* Inline reply form */}
      {replyOpen && (
        <div className="mt-3 space-y-2">
          <textarea
            value={replyBody}
            onChange={(e) => setReplyBody(e.target.value)}
            placeholder="Write a reply… Markdown supported."
            rows={3}
            className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground resize-none"
          />
          <div className="flex gap-2">
            <button
              type="button"
              disabled={!replyBody.trim() || replyMutation.isPending}
              onClick={() => replyMutation.mutate(replyBody.trim())}
              className="px-3 py-1.5 text-xs font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
            >
              {replyMutation.isPending ? "Posting…" : "Post reply"}
            </button>
            <button
              type="button"
              onClick={() => {
                setReplyOpen(false);
                setReplyBody("");
              }}
              className="px-3 py-1.5 text-xs font-medium rounded-lg bg-accent text-muted-foreground hover:bg-muted transition-colors"
            >
              Cancel
            </button>
          </div>
          {replyMutation.isError && (
            <p className="text-xs text-red-500">
              {(replyMutation.error as Error).message}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// ──────────────────────────────────────────────────────────────────────────────

export default function ContractComments({
  contractId,
}: ContractCommentsProps) {
  const queryClient = useQueryClient();
  const [commentBody, setCommentBody] = useState("");
  const [previewMode, setPreviewMode] = useState(false);

  const { data, isLoading, error } = useQuery({
    queryKey: ["contract-comments", contractId],
    queryFn: () => api.getComments(contractId),
  });

  const postMutation = useMutation({
    mutationFn: (body: string) => api.postComment(contractId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["contract-comments", contractId],
      });
      setCommentBody("");
      setPreviewMode(false);
    },
  });

  // Build comment tree: root comments + their replies
  const { roots, repliesFor } = useMemo(() => {
    const all = data?.items ?? [];
    const roots = all.filter((c) => c.parent_id === null);
    const repliesFor: Record<string, Comment[]> = {};
    for (const c of all) {
      if (c.parent_id) {
        if (!repliesFor[c.parent_id]) repliesFor[c.parent_id] = [];
        repliesFor[c.parent_id].push(c);
      }
    }
    return { roots, repliesFor };
  }, [data]);

  return (
    <section className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-foreground flex items-center gap-2">
          <MessageSquare className="w-5 h-5" />
          Discussion
          {data && data.total > 0 && (
            <span className="text-sm font-normal text-muted-foreground">
              ({data.total})
            </span>
          )}
        </h2>
      </div>

      {/* Compose form */}
      <div className="rounded-xl border border-border bg-card p-4 space-y-3">
        <div className="flex gap-2 border-b border-border pb-2">
          <button
            type="button"
            onClick={() => setPreviewMode(false)}
            className={`text-xs font-medium px-2 py-1 rounded transition-colors ${
              !previewMode
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Write
          </button>
          <button
            type="button"
            onClick={() => setPreviewMode(true)}
            className={`text-xs font-medium px-2 py-1 rounded transition-colors ${
              previewMode
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Preview
          </button>
        </div>

        {previewMode ? (
          <div className="min-h-[80px] text-sm text-foreground leading-relaxed">
            {commentBody.trim() ? (
              renderMarkdown(commentBody)
            ) : (
              <span className="text-muted-foreground text-xs">
                Nothing to preview.
              </span>
            )}
          </div>
        ) : (
          <textarea
            value={commentBody}
            onChange={(e) => setCommentBody(e.target.value)}
            placeholder="Share feedback, report issues, or ask questions. Markdown supported."
            rows={4}
            className="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground resize-none"
          />
        )}

        <div className="flex items-center justify-between">
          <p className="text-xs text-muted-foreground">
            Markdown — **bold**, _italic_, `code`, fenced blocks
          </p>
          <button
            type="button"
            disabled={!commentBody.trim() || postMutation.isPending}
            onClick={() => postMutation.mutate(commentBody.trim())}
            className="px-4 py-2 text-sm font-medium rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-colors"
          >
            {postMutation.isPending ? "Posting…" : "Post comment"}
          </button>
        </div>

        {postMutation.isError && (
          <p className="text-xs text-red-500">
            {(postMutation.error as Error).message}
          </p>
        )}
      </div>

      {/* Comments list */}
      {isLoading && (
        <div className="space-y-3">
          {[1, 2, 3].map((n) => (
            <div key={n} className="h-20 rounded-xl bg-muted animate-pulse" />
          ))}
        </div>
      )}

      {error && (
        <div className="rounded-xl border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-500">
          Failed to load comments.
        </div>
      )}

      {!isLoading && !error && roots.length === 0 && (
        <p className="text-sm text-muted-foreground py-4 text-center">
          No comments yet. Be the first to start the discussion.
        </p>
      )}

      {!isLoading && !error && roots.length > 0 && (
        <div className="space-y-4">
          {roots.map((comment) => (
            <div key={comment.id} className="space-y-2">
              <CommentCard comment={comment} contractId={contractId} />
              {(repliesFor[comment.id] ?? []).map((reply) => (
                <CommentCard
                  key={reply.id}
                  comment={reply}
                  contractId={contractId}
                  isReply
                />
              ))}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
