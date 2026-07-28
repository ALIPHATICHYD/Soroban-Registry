"use client";

import React, { useMemo } from "react";
import { CheckCircle2, Clock, XCircle } from "lucide-react";
import type { StatusEvent, VerificationStatus } from "@/types/verification";

type TimelineNode = {
  key: VerificationStatus;
  label: string;
  description: string;
};

const BASE_TIMELINE: TimelineNode[] = [
  {
    key: "submitted",
    label: "Submitted",
    description: "Your verification request was received.",
  },
  {
    key: "under_review",
    label: "Under review",
    description: "Reviewers evaluate your contract and documents.",
  },
  {
    key: "approved",
    label: "Approved",
    description: "Your contract is verified and badged.",
  },
];

export default function StatusTracker(props: {
  status: VerificationStatus;
  history: StatusEvent[];
}) {
  const { status, history } = props;

  const timeline = useMemo(() => {
    if (status === "rejected") {
      return [
        {
          key: "submitted",
          label: "Submitted",
          description: "Your verification request was received.",
        },
        {
          key: "under_review",
          label: "Under review",
          description: "Reviewers evaluate your contract and documents.",
        },
        {
          key: "rejected",
          label: "Rejected",
          description: "Reviewers rejected the verification request.",
        },
      ] as TimelineNode[];
    }
    if (status === "draft") return BASE_TIMELINE;
    return BASE_TIMELINE;
  }, [status]);

  const completed = new Set(history.map((h) => h.status));

  return (
    <div className="rounded-2xl border border-border bg-card p-4">
      <h3 className="text-sm font-semibold text-foreground">Progress</h3>
      <ol className="mt-4 space-y-3">
        {timeline.map((node) => {
          const isDone =
            completed.has(node.key) ||
            (status === "approved" && node.key !== "rejected");
          const isCurrent = status === node.key;

          const Icon =
            node.key === "rejected" ? XCircle : isDone ? CheckCircle2 : Clock;
          const color =
            node.key === "rejected"
              ? "text-red-500"
              : isDone
                ? "text-green-500"
                : isCurrent
                  ? "text-primary"
                  : "text-muted-foreground";

          return (
            <li key={node.key} className="flex items-start gap-3">
              <div className="pt-0.5">
                <Icon className={`w-5 h-5 ${color}`} />
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <p
                    className={`text-sm font-semibold ${isCurrent ? "text-foreground" : "text-muted-foreground"}`}
                  >
                    {node.label}
                  </p>
                  {isCurrent && (
                    <span className="text-[11px] text-primary font-semibold uppercase tracking-wide">
                      Current
                    </span>
                  )}
                </div>
                <p className="text-xs text-muted-foreground">
                  {node.description}
                </p>
              </div>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
