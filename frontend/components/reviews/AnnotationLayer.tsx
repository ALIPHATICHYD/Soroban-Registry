"use client";

import type { ReactNode } from "react";
import { MessageSquarePlus } from "lucide-react";

type AnnotationLocation = { line?: number; abi_path?: string };

interface AnnotationLayerProps {
  children: ReactNode;
  onAnnotate: (location: AnnotationLocation) => void;
  annotations: AnnotationLocation[];
}

export default function AnnotationLayer({
  children,
  onAnnotate,
  annotations,
}: AnnotationLayerProps) {
  return (
    <div className="relative group/layer">
      {children}

      {/* This is a conceptual overlay. In a real implementation with HighlightedRustCode, 
          we would inject these into the line rendering logic. 
          For now, we'll provide a way to click on lines if they are wrapped. */}
    </div>
  );
}

// Helper to wrap lines with annotation triggers
export function AnnotationWrapper({
  children,
  line,
  abiPath,
  onAnnotate,
  hasAnnotation,
}: {
  children: ReactNode;
  line?: number;
  abiPath?: string;
  onAnnotate: (loc: AnnotationLocation) => void;
  hasAnnotation?: boolean;
}) {
  return (
    <div className="relative group/line flex items-center gap-2 hover:bg-primary/5 transition-colors">
      <button
        onClick={() => onAnnotate({ line, abi_path: abiPath })}
        className={`opacity-0 group-hover/line:opacity-100 p-1 hover:text-primary transition-all ${
          hasAnnotation ? "opacity-100 text-primary" : "text-muted-foreground"
        }`}
      >
        <MessageSquarePlus className="w-4 h-4" />
      </button>
      <div className="flex-1">{children}</div>
    </div>
  );
}
