"use client";

import { useContext } from "react";
import { RealtimeContext } from "@/providers/RealtimeProvider";

export function useRealtime() {
  const context = useContext(RealtimeContext);

  // Fallback for SSR or when provider is not available
  const fallback = {
    isConnected: false,
    unreadCount: 0,
    notifications: [],
    subscribe: () => () => {},
    clearNotifications: () => {},
    markAsRead: () => {},
  };

  if (!context) {
    return fallback;
  }

  return context;
}
