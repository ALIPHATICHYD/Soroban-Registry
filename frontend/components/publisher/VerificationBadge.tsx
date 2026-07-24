import React from "react";
import { CheckCircle, XCircle, Clock } from "lucide-react";

interface VerificationBadgeProps {
  status: "verified" | "failed" | "pending";
}

export function VerificationBadge({ status }: VerificationBadgeProps) {
  const config = {
    verified: {
      icon: CheckCircle,
      text: "Verified",
      className:
        "bg-green-100 text-green-800 border-green-200 dark:bg-green-900/30 dark:text-green-300 dark:border-green-800",
    },
    failed: {
      icon: XCircle,
      text: "Failed",
      className:
        "bg-red-100 text-red-800 border-red-200 dark:bg-red-900/30 dark:text-red-300 dark:border-red-800",
    },
    pending: {
      icon: Clock,
      text: "Pending",
      className:
        "bg-yellow-100 text-yellow-800 border-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-300 dark:border-yellow-800",
    },
  };

  const { icon: Icon, text, className } = config[status] || config.pending;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border ${className}`}
    >
      <Icon className="w-3.5 h-3.5" />
      {text}
    </span>
  );
}
