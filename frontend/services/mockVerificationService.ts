import type {
  VerificationLogEntry,
  VerificationLogLevel,
  VerificationMetrics,
  StatusEvent,
  VerificationRequest,
  VerificationStatus,
  VerificationStatusChangeEvent,
  VerificationStatusResponse,
  VerificationSubmission,
} from "@/types/verification";

const STORAGE_KEY = "sr_verification_requests_v1";
const EVENT_NAME = "sr_verification_status_changed";

type StoredRequest = VerificationRequest & {
  // Stores future status transitions so refreshes can resume the same progression.
  scheduledTransitions?: Array<{
    status: VerificationStatus;
    atMs: number;
    message?: string;
  }>;
};

function nowIso(): string {
  return new Date().toISOString();
}

function safeParse<T>(raw: string | null): T | null {
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

function readAll(): StoredRequest[] {
  if (typeof window === "undefined") return [];
  const parsed = safeParse<StoredRequest[]>(
    window.localStorage.getItem(STORAGE_KEY),
  );
  if (!Array.isArray(parsed)) return [];
  return parsed.map((item) => {
    const createdAt = item.createdAt || nowIso();
    return {
      ...item,
      logs: Array.isArray(item.logs) ? item.logs : [],
      metrics: item.metrics || baseMetrics(createdAt),
      statusHistory: Array.isArray(item.statusHistory)
        ? item.statusHistory
        : [],
    };
  });
}

function writeAll(next: StoredRequest[]): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
}

function updateRequest(
  id: string,
  updater: (prev: StoredRequest) => StoredRequest,
): StoredRequest | null {
  const all = readAll();
  const idx = all.findIndex((r) => r.id === id);
  if (idx === -1) return null;
  const updated = updater(all[idx]);
  all[idx] = updated;
  writeAll(all);
  return updated;
}

function hashToUnitInterval(input: string): number {
  let h = 2166136261;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0) / 2 ** 32;
}

function dispatchStatusChange(payload: VerificationStatusChangeEvent): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent<VerificationStatusChangeEvent>(EVENT_NAME, {
      detail: payload,
    }),
  );
}

function makeLogEntry(params: {
  level: VerificationLogLevel;
  phase: VerificationLogEntry["phase"];
  message: string;
  output?: string;
}): VerificationLogEntry {
  return {
    id: `log_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`,
    at: nowIso(),
    level: params.level,
    phase: params.phase,
    message: params.message,
    output: params.output,
  };
}

function baseMetrics(createdAt: string): VerificationMetrics {
  return {
    attemptCount: 1,
    checksPassed: 0,
    checksFailed: 0,
    durationMs: 0,
    coveragePct: 0,
    lastUpdatedAt: createdAt,
  };
}

export function subscribeToVerificationStatusChanges(
  handler: (evt: VerificationStatusChangeEvent) => void,
): () => void {
  if (typeof window === "undefined") return () => undefined;

  const listener = (e: Event) => {
    const ce = e as CustomEvent<VerificationStatusChangeEvent>;
    if (!ce.detail) return;
    handler(ce.detail);
  };

  window.addEventListener(EVENT_NAME, listener);
  return () => window.removeEventListener(EVENT_NAME, listener);
}

function pushStatus(
  request: StoredRequest,
  nextStatus: VerificationStatus,
  message?: string,
): StoredRequest {
  const next: StatusEvent = { status: nextStatus, at: nowIso(), message };
  const statusLog = makeLogEntry({
    level: nextStatus === "rejected" ? "error" : "info",
    phase:
      nextStatus === "submitted"
        ? "submission"
        : nextStatus === "under_review"
          ? "review"
          : "decision",
    message: message || `Status changed to ${nextStatus}`,
    output: JSON.stringify({ status: nextStatus, at: next.at }, null, 2),
  });
  const elapsedMs = Math.max(
    100,
    new Date(next.at).getTime() - new Date(request.createdAt).getTime(),
  );
  const checksPassed =
    nextStatus === "approved"
      ? 16
      : nextStatus === "under_review"
        ? 8
        : request.metrics.checksPassed;
  const checksFailed =
    nextStatus === "rejected"
      ? Math.max(1, request.metrics.checksFailed)
      : request.metrics.checksFailed;
  const coveragePct =
    nextStatus === "approved"
      ? 100
      : nextStatus === "under_review"
        ? 82
        : request.metrics.coveragePct;
  return {
    ...request,
    status: nextStatus,
    updatedAt: next.at,
    statusHistory: [...request.statusHistory, next],
    logs: [...request.logs, statusLog],
    metrics: {
      ...request.metrics,
      checksPassed,
      checksFailed,
      durationMs: elapsedMs,
      coveragePct,
      lastUpdatedAt: next.at,
    },
    lastErrorDetails:
      nextStatus === "rejected"
        ? "Static analyzer detected an unbounded authorization path in transfer() under edge-case call ordering."
        : undefined,
  };
}

function ensureProgressionScheduled(request: StoredRequest): StoredRequest {
  if (request.scheduledTransitions && request.scheduledTransitions.length > 0)
    return request;
  if (request.status === "approved" || request.status === "rejected")
    return request;

  // Deterministic mock outcome per request id so "approved vs rejected" is stable across reloads.
  const baseMs = Date.now();
  const approvalChance = 0.75;
  const unit = hashToUnitInterval(request.id);
  const finalStatus: VerificationStatus =
    unit < approvalChance ? "approved" : "rejected";

  const transitions: StoredRequest["scheduledTransitions"] = [
    {
      status: "under_review",
      atMs: baseMs + 2500,
      message: "Verification is now under review.",
    },
    {
      status: finalStatus,
      atMs: baseMs + 9000,
      message:
        finalStatus === "approved"
          ? "Verification approved."
          : "Verification rejected.",
    },
  ];

  return { ...request, scheduledTransitions: transitions };
}

function applyDueTransitions(request: StoredRequest): StoredRequest {
  if (
    !request.scheduledTransitions ||
    request.scheduledTransitions.length === 0
  )
    return request;

  const now = Date.now();
  let current = request;
  const remaining: NonNullable<StoredRequest["scheduledTransitions"]> = [];

  for (const t of request.scheduledTransitions) {
    if (t.atMs <= now && current.status !== t.status) {
      current = pushStatus(current, t.status, t.message);
    } else if (t.atMs > now) {
      remaining.push(t);
    }
  }

  return { ...current, scheduledTransitions: remaining };
}

export async function submitVerification(params: {
  submission: VerificationSubmission;
}): Promise<VerificationRequest> {
  // Mock server assigns a request id and immediately moves status to `submitted`.
  const id = `vr_${Math.random().toString(36).slice(2, 10)}_${Date.now().toString(36)}`;
  const createdAt = nowIso();

  const request: StoredRequest = {
    id,
    createdAt,
    updatedAt: createdAt,
    status: "submitted",
    submission: params.submission,
    statusHistory: [
      {
        status: "submitted",
        at: createdAt,
        message: "Verification submitted.",
      },
    ],
    logs: [
      makeLogEntry({
        level: "info",
        phase: "submission",
        message: "Submission accepted and queued.",
        output: JSON.stringify(
          {
            network: params.submission.network,
            documents: params.submission.documents.length,
            riskLevel: params.submission.riskLevel,
          },
          null,
          2,
        ),
      }),
      makeLogEntry({
        level: "debug",
        phase: "precheck",
        message: "Running deterministic mock validators.",
        output:
          "validateContractAddress=true\nvalidateNetwork=true\nvalidateDocumentSet=true",
      }),
    ],
    metrics: baseMetrics(createdAt),
  };

  const all = readAll();
  writeAll([request, ...all]);

  await new Promise((r) => setTimeout(r, 650));
  dispatchStatusChange({ id, status: "submitted" });
  return request;
}

export async function getVerificationStatus(params: {
  id: string;
}): Promise<VerificationStatusResponse> {
  const all = readAll();
  const found = all.find((r) => r.id === params.id);
  if (!found) {
    throw new Error("Verification request not found");
  }

  // Apply any transitions that should have happened while the user was away.
  const scheduled = ensureProgressionScheduled(found);
  const progressed = applyDueTransitions(scheduled);
  if (
    progressed.updatedAt !== found.updatedAt ||
    progressed.scheduledTransitions !== found.scheduledTransitions
  ) {
    updateRequest(params.id, () => progressed);
  }

  await new Promise((r) => setTimeout(r, 250));
  return { request: progressed };
}

export function simulateStatusProgression(params: { id: string }): () => void {
  let active = true;
  const timeouts: number[] = [];

  const schedule = () => {
    const request = updateRequest(params.id, (prev) =>
      ensureProgressionScheduled(prev),
    );
    if (
      !request?.scheduledTransitions ||
      request.scheduledTransitions.length === 0
    )
      return;

    const remaining = request.scheduledTransitions;
    for (const t of remaining) {
      const delay = Math.max(0, t.atMs - Date.now());
      const timeoutId = window.setTimeout(() => {
        if (!active) return;
        const updated = updateRequest(params.id, (prev) =>
          applyDueTransitions(prev),
        );
        if (updated)
          dispatchStatusChange({ id: params.id, status: updated.status });
      }, delay);
      timeouts.push(timeoutId);
    }
  };

  schedule();

  return () => {
    active = false;
    for (const id of timeouts) window.clearTimeout(id);
  };
}

export async function retryVerification(params: {
  id: string;
}): Promise<VerificationRequest> {
  const now = nowIso();
  const updated = updateRequest(params.id, (prev) => {
    const cleared: StoredRequest = {
      ...prev,
      status: "submitted",
      updatedAt: now,
      statusHistory: [
        ...prev.statusHistory,
        { status: "submitted", at: now, message: "Retry requested." },
      ],
      scheduledTransitions: [],
      logs: [
        ...prev.logs,
        makeLogEntry({
          level: "warn",
          phase: "retry",
          message: "Retry initiated after failure.",
          output: prev.lastErrorDetails
            ? `previous_error="${prev.lastErrorDetails}"`
            : 'previous_error="n/a"',
        }),
      ],
      metrics: {
        ...prev.metrics,
        attemptCount: prev.metrics.attemptCount + 1,
        checksFailed: 0,
        coveragePct: 0,
        durationMs: 0,
        lastUpdatedAt: now,
      },
      lastErrorDetails: undefined,
    };
    return ensureProgressionScheduled(cleared);
  });

  if (!updated) throw new Error("Unable to retry verification request.");
  await new Promise((r) => setTimeout(r, 300));
  dispatchStatusChange({ id: params.id, status: "submitted" });
  return updated;
}
