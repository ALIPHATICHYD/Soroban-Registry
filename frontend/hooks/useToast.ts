"use client";

import { useContext } from "react";
import { ToastContext, ToastContextValue } from "@/providers/ToastProvider";

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);

  if (!context) {
    // During SSR prerendering there is no ToastProvider in scope.
    // Return a no-op fallback — all real call-sites are 'use client'
    // components so this path is never hit at runtime.
    const noop = () => {};
    return {
      toasts: [],
      showToast: noop,
      dismissToast: noop,
      showError: noop,
      showSuccess: noop,
      showWarning: noop,
      showInfo: noop,
    };
  }

  return context;
}
