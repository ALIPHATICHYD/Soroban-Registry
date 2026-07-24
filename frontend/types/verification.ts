export type VerificationStatus =
  | "unverified"
  | "draft"
  | "submitted"
  | "under_review"
  | "approved"
  | "rejected";
export type VerificationLevel = "basic" | "intermediate" | "advanced";
export type VerificationLogLevel = "info" | "warn" | "error" | "debug";

export type SorobanNetwork = "mainnet" | "testnet" | "futurenet";

export type RiskLevel = "low" | "medium" | "high" | "critical";

export type AuditStatus = "not_audited" | "in_progress" | "audited";

export type VerificationStepKey =
  | "contractInfo"
  | "description"
  | "securityClaims"
  | "documents"
  | "review";

export type VerificationDocument = {
  id: string;
  name: string;
  sizeBytes: number;
  mimeType: string;
  uploadedAt: string;
};

export type VerificationDraft = {
  contractName: string;
  contractAddress: string;
  network: SorobanNetwork;

  purpose: string;
  useCase: string;

  auditStatus: AuditStatus;
  knownVulnerabilities: string;
  riskLevel: RiskLevel;
};

export type VerificationSubmission = VerificationDraft & {
  documents: VerificationDocument[];
};

export type StatusEvent = {
  status: VerificationStatus;
  at: string;
  message?: string;
};

export type VerificationLogEntry = {
  id: string;
  at: string;
  level: VerificationLogLevel;
  phase:
    | "submission"
    | "precheck"
    | "analysis"
    | "review"
    | "decision"
    | "retry";
  message: string;
  output?: string;
};

export type VerificationMetrics = {
  attemptCount: number;
  checksPassed: number;
  checksFailed: number;
  durationMs: number;
  coveragePct: number;
  lastUpdatedAt: string;
};

export type VerificationRequest = {
  id: string;
  createdAt: string;
  updatedAt: string;
  status: VerificationStatus;
  submission: VerificationSubmission;
  statusHistory: StatusEvent[];
  logs: VerificationLogEntry[];
  metrics: VerificationMetrics;
  lastErrorDetails?: string;
};

export type VerificationStatusResponse = {
  request: VerificationRequest;
};

export type VerificationStatusChangeEvent = {
  id: string;
  status: VerificationStatus;
};
