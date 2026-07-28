"use client";

import { useState } from "react";
import { useAnalytics } from "@/hooks/useAnalytics";
import { useToast } from "@/hooks/useToast";

interface CopyOptions {
  successEventName?: string;
  failureEventName?: string;
  analyticsParams?: Record<string, unknown>;
  successMessage?: string;
  failureMessage?: string;
}

function fallbackCopyText(text: string): boolean {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.top = "-9999px";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.focus();
  textarea.select();

  try {
    return document.execCommand("copy");
  } finally {
    document.body.removeChild(textarea);
  }
}

export function useCopy() {
  const [copied, setCopied] = useState(false);
  const [isCopying, setIsCopying] = useState(false);
  const { logEvent } = useAnalytics();
  const { showSuccess, showError } = useToast();

  const copy = async (text: string, options?: CopyOptions) => {
    if (!text) {
      return false;
    }

    setIsCopying(true);

    try {
      if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const copiedWithFallback = fallbackCopyText(text);
        if (!copiedWithFallback) {
          throw new Error("Clipboard unavailable");
        }
      }

      setCopied(true);
      logEvent(options?.successEventName || "code_copied", {
        ...options?.analyticsParams,
      });
      showSuccess(options?.successMessage || "Copied to clipboard");

      setTimeout(() => setCopied(false), 1800);
      return true;
    } catch {
      logEvent(options?.failureEventName || "code_copy_failed", {
        ...options?.analyticsParams,
      });
      showError(options?.failureMessage || "Unable to copy to clipboard");
      return false;
    } finally {
      setIsCopying(false);
    }
  };

  return { copy, copied, isCopying };
}
