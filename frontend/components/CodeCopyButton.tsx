"use client";

import type { KeyboardEvent } from "react";
import { Check, Copy } from "lucide-react";

interface CodeCopyButtonProps {
  onCopy: () => void | Promise<void> | Promise<boolean>;
  copied: boolean;
  disabled?: boolean;
  idleLabel?: string;
  copiedLabel?: string;
  shortcutHint?: boolean;
  className?: string;
}

export default function CodeCopyButton({
  onCopy,
  copied,
  disabled = false,
  idleLabel = "Copy",
  copiedLabel = "Copied",
  shortcutHint = true,
  className = "",
}: CodeCopyButtonProps) {
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const isCopyShortcut =
      (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "c";
    if (!isCopyShortcut || disabled) return;
    event.preventDefault();
    void onCopy();
  };

  const shortcutLabel = shortcutHint ? " (Ctrl/Cmd+C)" : "";

  return (
    <button
      type="button"
      onClick={() => void onCopy()}
      onKeyDown={handleKeyDown}
      disabled={disabled}
      className={`inline-flex items-center gap-1 rounded-md border border-border bg-card px-2.5 py-1 text-xs font-medium text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60 ${className}`.trim()}
      aria-label={`${copied ? copiedLabel : idleLabel}${shortcutLabel}`}
      title={`${copied ? copiedLabel : idleLabel}${shortcutLabel}`}
    >
      {copied ? (
        <Check className="h-3.5 w-3.5" />
      ) : (
        <Copy className="h-3.5 w-3.5" />
      )}
      {copied ? copiedLabel : idleLabel}
    </button>
  );
}
