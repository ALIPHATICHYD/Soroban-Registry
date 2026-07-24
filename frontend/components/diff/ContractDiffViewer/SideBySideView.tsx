"use client";

import React from "react";
import { MessageSquare } from "lucide-react";
import { HighlightedLine } from "./HighlightedLine";
import { CommentThread, type DiffComment } from "./CommentThread";

export type SideBySideRow =
  | { kind: "context"; lineNo: number; value: string }
  | {
      kind: "change";
      leftNo: number | null;
      leftValue: string | null;
      rightNo: number | null;
      rightValue: string | null;
    };

interface SideBySideViewProps {
  rows: SideBySideRow[];
  comments: Record<string, DiffComment[]>;
  openThread: string | null;
  onToggleThread: (key: string) => void;
  onAddComment: (lineKey: string, text: string) => void;
}

export function SideBySideView({
  rows,
  comments,
  openThread,
  onToggleThread,
  onAddComment,
}: SideBySideViewProps) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse font-mono text-xs leading-5 table-fixed">
        <colgroup>
          <col className="w-8" />
          <col className="w-[calc(50%-2.5rem)]" />
          <col className="w-5" />
          <col className="w-8" />
          <col className="w-[calc(50%-2.5rem)]" />
          <col className="w-5" />
        </colgroup>
        <thead>
          <tr className="border-b border-border text-muted-foreground text-xs">
            <th colSpan={3} className="py-1 px-2 text-left font-semibold">
              Before
            </th>
            <th
              colSpan={3}
              className="py-1 px-2 text-left font-semibold border-l border-border"
            >
              After
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, idx) => {
            if (row.kind === "context") {
              return (
                <tr
                  key={idx}
                  className="hover:brightness-95 dark:hover:brightness-110"
                >
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums">
                    {row.lineNo}
                  </td>
                  <td className="py-0.5 pr-2 text-foreground" colSpan={2}>
                    <HighlightedLine value={row.value} />
                  </td>
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums border-l border-border/40">
                    {row.lineNo}
                  </td>
                  <td className="py-0.5 pr-2 text-foreground" colSpan={2}>
                    <HighlightedLine value={row.value} />
                  </td>
                </tr>
              );
            }

            const leftKey = row.leftNo !== null ? `${row.leftNo}-left` : null;
            const rightKey =
              row.rightNo !== null ? `${row.rightNo}-right` : null;
            const leftHasComments = leftKey
              ? (comments[leftKey]?.length ?? 0) > 0
              : false;
            const rightHasComments = rightKey
              ? (comments[rightKey]?.length ?? 0) > 0
              : false;

            return (
              <React.Fragment key={idx}>
                <tr className="group">
                  {/* Left (remove) */}
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums bg-red-500/10">
                    {row.leftNo ?? ""}
                  </td>
                  <td
                    className={`py-0.5 pr-1 ${row.leftValue !== null ? "bg-red-500/10 text-red-700 dark:text-red-300" : "bg-background"}`}
                  >
                    {row.leftValue !== null && (
                      <>
                        <span className="text-red-500 select-none mr-1">-</span>
                        <HighlightedLine value={row.leftValue} />
                      </>
                    )}
                  </td>
                  <td
                    className={`pl-1 ${row.leftValue !== null ? "bg-red-500/10" : "bg-background"}`}
                  >
                    {leftKey && (
                      <button
                        type="button"
                        onClick={() => onToggleThread(leftKey)}
                        className={`opacity-0 group-hover:opacity-100 transition-opacity rounded p-0.5 hover:bg-primary/10 ${leftHasComments ? "!opacity-100 text-primary" : "text-muted-foreground"}`}
                        title="Comment"
                      >
                        <MessageSquare size={12} />
                      </button>
                    )}
                  </td>

                  {/* Right (add) */}
                  <td className="px-2 text-right text-muted-foreground select-none border-r border-border/40 tabular-nums bg-green-500/10 border-l border-border/40">
                    {row.rightNo ?? ""}
                  </td>
                  <td
                    className={`py-0.5 pr-1 ${row.rightValue !== null ? "bg-green-500/10 text-green-700 dark:text-green-300" : "bg-background"}`}
                  >
                    {row.rightValue !== null && (
                      <>
                        <span className="text-green-500 select-none mr-1">
                          +
                        </span>
                        <HighlightedLine value={row.rightValue} />
                      </>
                    )}
                  </td>
                  <td
                    className={`pl-1 ${row.rightValue !== null ? "bg-green-500/10" : "bg-background"}`}
                  >
                    {rightKey && (
                      <button
                        type="button"
                        onClick={() => onToggleThread(rightKey)}
                        className={`opacity-0 group-hover:opacity-100 transition-opacity rounded p-0.5 hover:bg-primary/10 ${rightHasComments ? "!opacity-100 text-primary" : "text-muted-foreground"}`}
                        title="Comment"
                      >
                        <MessageSquare size={12} />
                      </button>
                    )}
                  </td>
                </tr>
                {(openThread === leftKey || openThread === rightKey) && (
                  <tr key={`${idx}-comment`}>
                    <td colSpan={6}>
                      <CommentThread
                        lineKey={openThread!}
                        comments={comments[openThread!] ?? []}
                        onAdd={onAddComment}
                        onClose={() => onToggleThread(openThread!)}
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
