"use client";

import React from 'react';
import { CheckCircle2, Info, ShieldAlert, ShieldCheck, ShieldX } from 'lucide-react';
import type { VerificationStatus } from '@/types/verification';
import { useTranslation } from '@/lib/i18n/client';
import type { TFunction } from 'i18next';

function getBadgeConfig(
  status: VerificationStatus,
  t: TFunction,
  level?: string,
): {
  label: string;
  className: string;
  Icon: React.ComponentType<{ className?: string }>;
} {
  switch (status) {
    case "approved": {
      const levelLabel = level
        ? ` (${level.charAt(0).toUpperCase()}${level.slice(1)})`
        : "";
      return {
        label: `${t("verificationBadge.verified")}${levelLabel}`,
        className: "bg-green-500/10 text-green-500 border-green-500/20",
        Icon: CheckCircle2,
      };
    }
    case "under_review":
      return {
        label: t("verificationBadge.underReview"),
        className: "bg-blue-500/10 text-blue-500 border-blue-500/20",
        Icon: ShieldCheck,
      };
    case "rejected":
      return {
        label: t("verificationBadge.rejected"),
        className: "bg-red-500/10 text-red-500 border-red-500/20",
        Icon: ShieldX,
      };
    case "submitted":
      return {
        label: t("verificationBadge.submitted"),
        className: "bg-yellow-500/10 text-yellow-600 border-yellow-500/20",
        Icon: ShieldAlert,
      };
    default:
      return {
        label: t("verificationBadge.draft"),
        className: "bg-muted text-muted-foreground border-border",
        Icon: ShieldAlert,
      };
  }
}

interface VerificationBadgeProps {
  status: VerificationStatus;
  level?: string;
  size?: "sm" | "md";
}

export default function VerificationBadge({
  status,
  level,
  size = "sm",
}: VerificationBadgeProps) {
  const { t } = useTranslation("common");
  const cfg = getBadgeConfig(status, t, level);

  const iconSize = size === "md" ? "w-4 h-4" : "w-3 h-3";
  const textSize = size === "md" ? "text-xs" : "text-[10px]";
  const padding = size === "md" ? "px-2.5 py-1" : "px-2 py-0.5";

  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full border ${padding} ${textSize} font-semibold uppercase tracking-wide ${cfg.className}`}
    >
      <cfg.Icon className={iconSize} />
      {cfg.label}
    </span>
  );
}
