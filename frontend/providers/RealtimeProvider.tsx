"use client";

import {
  createContext,
  useCallback,
  useEffect,
  useState,
  ReactNode,
} from "react";
import { wsService } from "@/services/websocket.service";
import type { ContractDeploymentEvent, RealtimeContextType } from "@/types";
import {
  requestDesktopNotification,
  sendDesktopNotification,
} from "@/utils/notifications";

export const RealtimeContext = createContext<RealtimeContextType | undefined>(
  undefined,
);

interface RealtimeProviderProps {
  children: ReactNode;
}

export default function RealtimeProvider({ children }: RealtimeProviderProps) {
  const [isConnected, setIsConnected] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);
  const [notifications, setNotifications] = useState<ContractDeploymentEvent[]>(
    [],
  );

  useEffect(() => {
    // Request desktop notification permission on mount
    requestDesktopNotification();

    // Connect WebSocket
    wsService.connect().catch(console.error);

    // Handle connection
    const unsubscribeOpen = wsService.onOpen(() => {
      setIsConnected(true);
    });

    // Handle deployment events
    const unsubscribeDeploy = wsService.on(
      "contract_deployed",
      (data: unknown) => {
        const typedData = data as ContractDeploymentEvent;
        setNotifications((prev) => [typedData, ...prev].slice(0, 50)); // Keep last 50
        setUnreadCount((prev) => prev + 1);

        // Show desktop notification if enabled
        const preferences = localStorage.getItem("notification-preferences");
        if (preferences) {
          const prefs = JSON.parse(preferences);
          if (
            prefs.enableDesktopNotifications &&
            prefs.enableDeploymentNotifications
          ) {
            sendDesktopNotification("New Contract Deployed", {
              body: `${typedData.contractName} v${typedData.version} by ${typedData.publisher}`,
              tag: `contract-${typedData.contractId}`,
              requireInteraction: false,
            });
          }
        }
      },
    );

    // Handle update events
    const unsubscribeUpdate = wsService.on(
      "contract_updated",
      (data: unknown) => {
        const typedData = data as Record<string, unknown>;
        const preferences = localStorage.getItem("notification-preferences");
        if (preferences) {
          const prefs = JSON.parse(preferences);
          if (
            prefs.enableDesktopNotifications &&
            prefs.enableUpdateNotifications &&
            prefs.updateTypes.includes(typedData.updateType)
          ) {
            sendDesktopNotification(
              `Contract Update: ${typedData.updateType}`,
              {
                body: `Contract ${typedData.contractId} has been updated`,
                tag: `update-${typedData.contractId}`,
              },
            );
          }
        }
      },
    );

    return () => {
      unsubscribeOpen();
      unsubscribeDeploy();
      unsubscribeUpdate();
      wsService.disconnect();
    };
  }, []);

  const subscribe = useCallback(
    (type: string, handler: (data: unknown) => void) => {
      return wsService.on(type, handler);
    },
    [],
  );

  const clearNotifications = useCallback(() => {
    setNotifications([]);
    setUnreadCount(0);
  }, []);

  const markAsRead = useCallback(() => {
    // Mark notification as read and decrement unread count
    if (unreadCount > 0) {
      setUnreadCount((prev) => prev - 1);
    }
  }, [unreadCount]);

  const value: RealtimeContextType = {
    isConnected,
    unreadCount,
    notifications,
    subscribe,
    clearNotifications,
    markAsRead,
  };

  return (
    <RealtimeContext.Provider value={value}>
      {children}
    </RealtimeContext.Provider>
  );
}
