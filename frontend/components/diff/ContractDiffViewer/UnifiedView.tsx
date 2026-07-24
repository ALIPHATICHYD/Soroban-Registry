"use client";

import React from "react";
import { MessageSquare } from "lucide-react";
import { HighlightedLine } from "./HighlightedLine";
import { CommentThread, type DiffComment } from "./CommentThread";
import { DiffLine } from "@/utils/comparison";

interface UnifiedViewProps {
  lines: DiffLine[];
  comments: Record<string, DiffComment[]>;
  openThread: string | null;
  onToggleThread: (key: string) => void;
  onAddComment: (lineKey: string, text: string) => void;
  onCloseThread: () => void;
}

export function UnifiedView({
  lines,
  comments,
  openThread,
  onToggleThread,
  onAddComment,
  onCloseThread,
}: UnifiedViewProps) {
  let leftNo = 1;
  let rightNo = 1;

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse font-mono text-xs leading-5">
        <colgroup>
          <col className="w-10" />
          <col className="w-10" />
          <col className="w-5" />
          <col />
          <col className="w-6" />
        </colgroup>
        <tbody>
          {lines.map((l, idx) => {
            const isAdd = l.type === "add";
            const isRemove = l.type === "remove";

            const ln = isAdd ? rightNo : isRemove ? leftNo : leftNo;
            const lineKey = `${ln}-${isAdd ? "right" : "left"}`;

            const rowClass = isAdd
              ? "bg-green-500/10"
              : isRemove
                ? "bg-red-500/10"
                : "";

            const prefix = isAdd ? "+" : isRemove ? "-" : " ";
            const prefixClass = isAdd
              ? "text-green-600 dark:text-green-400 select-none"
              : isRemove
                ? "text-red-600 dark:text-red-400 select-none"
                : "text-muted-foreground select-none";
            const textClass = isAdd
              ? "text-green-700 dark:text-green-300"
              : isRemove
                ? "text-red-700 dark:text-red-300"
                : "text-foreground";

            const leftLabel = isAdd ? "" : String(leftNo);
            const rightLabel = isRemove ? "" : String(rightNo);

            if (l.type === "context") {
              leftNo++;
              rightNo++;
            } else if (l.type === "add") rightNo++;
            else leftNo++;

            const hasComments = (comments[lineKey]?.length ?? 0) > 0;

            return (
              <React.Fragment key={idx}>
                <tr
                  className={`group ${rowClass} hover:brightness-95 dark:hover:brightness-110`}
                >
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums">
                    {leftLabel}
                  </td>
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums">
                    {rightLabel}
                  </td>
                  <td className={`px-1 text-center ${prefixClass}`}>
                    {prefix}
                  </td>
                  <td className={`py-0.5 pr-2 ${textClass}`}>
                    <HighlightedLine value={l.value} />
                  </td>
                  <td className="pl-1">
                    <button
                      type="button"
                      onClick={() => onToggleThread(lineKey)}
                      className={`opacity-0 group-hover:opacity-100 transition-opacity rounded p-0.5 hover:bg-primary/10 ${hasComments ? "!opacity-100 text-primary" : "text-muted-foreground"}`}
                      title="Comment on this line"
                    >
                      <MessageSquare size={12} />
                    </button>
                  </td>
                </tr>
                {openThread === lineKey && (
                  <tr key={`${idx}-comment`}>
                    <td colSpan={5}>
                      <CommentThread
                        lineKey={lineKey}
                        comments={comments[lineKey] ?? []}
                        onAdd={onAddComment}
                        onClose={onCloseThread}
                      />
                    </td>
                  </tr>
                )}
              </React.Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
