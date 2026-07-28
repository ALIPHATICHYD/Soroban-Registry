export interface ContractDeploymentEvent {
  contractId: string;
  contractName: string;
  publisher: string;
  timestamp: string;
  version: string;
}

export interface ContractUpdateEvent {
  contractId: string;
  updateType:
    | "verification_status"
    | "security_audit"
    | "deprecation"
    | "breaking_change";
  details: Record<string, unknown>;
  timestamp: string;
}

export type RealtimeEvent = ContractDeploymentEvent | ContractUpdateEvent;

export interface NotificationPreferences {
  enableDeploymentNotifications: boolean;
  enableUpdateNotifications: boolean;
  enableDesktopNotifications: boolean;
  enableSoundNotification: boolean;
  updateTypes: string[];
}

export interface RealtimeContextType {
  isConnected: boolean;
  unreadCount: number;
  notifications: ContractDeploymentEvent[];
  subscribe: (type: string, handler: (data: unknown) => void) => () => void;
  clearNotifications: () => void;
  markAsRead: (notificationId: string) => void;
}
