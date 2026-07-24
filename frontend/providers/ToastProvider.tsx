"use client";

import { createContext, useCallback, useState, ReactNode } from "react";
import ToastContainer from "@/components/ToastContainer";

export interface Toast {
  id: string;
  message: string;
  type: "error" | "warning" | "success" | "info";
  duration?: number;
  dismissible?: boolean;
}

export interface ToastContextValue {
  toasts: Toast[];
  showToast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: string) => void;
  showError: (message: string, duration?: number) => void;
  showSuccess: (message: string, duration?: number) => void;
  showWarning: (message: string, duration?: number) => void;
  showInfo: (message: string, duration?: number) => void;
}

export const ToastContext = createContext<ToastContextValue | undefined>(
  undefined,
);

interface ToastProviderProps {
  children: ReactNode;
}

export default function ToastProvider({ children }: ToastProviderProps) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [nextId, setNextId] = useState(0);

  const showToast = useCallback(
    (toast: Omit<Toast, "id">) => {
      const id = `toast-${nextId}`;
      setNextId((prev) => prev + 1);

      setToasts((prev) => [...prev, { ...toast, id }]);
    },
    [nextId],
  );

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
  }, []);

  const showError = useCallback(
    (message: string, duration = 5000) => {
      showToast({ message, type: "error", duration, dismissible: true });
    },
    [showToast],
  );

  const showSuccess = useCallback(
    (message: string, duration = 5000) => {
      showToast({ message, type: "success", duration, dismissible: true });
    },
    [showToast],
  );

  const showWarning = useCallback(
    (message: string, duration = 5000) => {
      showToast({ message, type: "warning", duration, dismissible: true });
    },
    [showToast],
  );

  const showInfo = useCallback(
    (message: string, duration = 5000) => {
      showToast({ message, type: "info", duration, dismissible: true });
    },
    [showToast],
  );

  const value: ToastContextValue = {
    toasts,
    showToast,
    dismissToast,
    showError,
    showSuccess,
    showWarning,
    showInfo,
  };

  return (
    <ToastContext.Provider value={value}>
      {children}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </ToastContext.Provider>
  );
}
