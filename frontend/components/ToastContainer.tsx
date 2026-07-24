"use client";

import Toast from "./Toast";

export interface ToastData {
  id: string;
  message: string;
  type: "error" | "warning" | "success" | "info";
  duration?: number;
  dismissible?: boolean;
}

interface ToastContainerProps {
  toasts: ToastData[];
  onDismiss: (id: string) => void;
}

export default function ToastContainer({
  toasts,
  onDismiss,
}: ToastContainerProps) {
  if (toasts.length === 0) return null;

  return (
    <div
      className="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none"
      aria-label="Notifications"
    >
      {toasts.map((toast) => (
        <div key={toast.id} className="pointer-events-auto">
          <Toast
            id={toast.id}
            message={toast.message}
            type={toast.type}
            duration={toast.duration}
            dismissible={toast.dismissible}
            onDismiss={onDismiss}
          />
        </div>
      ))}
    </div>
  );
}
