export type NetworkName = string;
export type CategoryName = string;

export type DeploymentCreatedEvent = {
  type: "deployment_created";
  payload: {
    id: string;
    contractId: string;
    network: NetworkName;
    category?: CategoryName;
    publisher?: string;
    timestamp: string;
  };
};

export type ContractInteractionEvent = {
  type: "contract_interaction";
  payload: {
    id: string;
    contractId: string;
    network: NetworkName;
    timestamp: string;
  };
};

export type NetworkStatusEvent = {
  type: "network_status";
  payload: {
    network: NetworkName;
    status: "connected" | "degraded" | "disconnected";
    latencyMs?: number;
    timestamp: string;
  };
};

export type RegistryEvent =
  | DeploymentCreatedEvent
  | ContractInteractionEvent
  | NetworkStatusEvent;

export type ConnectionState =
  | {
      status: "disconnected";
      wsUrl: string;
      lastError?: string;
      lastConnectedAt?: number;
    }
  | {
      status: "connected";
      wsUrl: string;
      connectedAt: number;
      latencyMs?: number;
    }
  | {
      status: "reconnecting";
      wsUrl: string;
      attempt: number;
      nextRetryAt: number;
      lastError?: string;
      lastConnectedAt?: number;
    };

export type DashboardFilters = {
  network?: NetworkName;
  category?: CategoryName;
  query?: string;
};

export type Deployment = {
  id: string;
  contractId: string;
  network: NetworkName;
  category?: CategoryName;
  publisher?: string;
  ts: number;
};

export type Interaction = {
  id: string;
  contractId: string;
  network: NetworkName;
  ts: number;
};

export type ActivityBucket = {
  startTs: number;
  deployments: number;
  interactions: number;
};

export type LoadingState = {
  deployments: boolean;
  trending: boolean;
  activity: boolean;
};

export type DashboardState = {
  connection: ConnectionState;
  filters: DashboardFilters;
  deployments: Deployment[];
  interactions: Interaction[];
  activity: ActivityBucket[];
  nowTs: number;
  loading: LoadingState;
};
