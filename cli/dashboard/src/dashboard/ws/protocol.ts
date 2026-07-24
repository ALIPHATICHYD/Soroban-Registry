import type { RegistryEvent } from "../types";

// Parses and validates incoming WebSocket messages (defensive against bad payloads).
export function parseRegistryEvent(
  message: unknown,
): RegistryEvent | undefined {
  if (!isRecord(message)) return undefined;
  const type = message.type;
  const payload = message.payload;
  if (typeof type !== "string" || !isRecord(payload)) return undefined;

  if (type === "deployment_created") {
    if (typeof payload.id !== "string") return undefined;
    if (typeof payload.contractId !== "string") return undefined;
    if (typeof payload.network !== "string") return undefined;
    if (typeof payload.timestamp !== "string") return undefined;
    if (payload.category !== undefined && typeof payload.category !== "string")
      return undefined;
    if (
      payload.publisher !== undefined &&
      typeof payload.publisher !== "string"
    )
      return undefined;
    return message as RegistryEvent;
  }

  if (type === "contract_interaction") {
    if (typeof payload.id !== "string") return undefined;
    if (typeof payload.contractId !== "string") return undefined;
    if (typeof payload.network !== "string") return undefined;
    if (typeof payload.timestamp !== "string") return undefined;
    return message as RegistryEvent;
  }

  if (type === "network_status") {
    if (typeof payload.network !== "string") return undefined;
    if (
      payload.status !== "connected" &&
      payload.status !== "degraded" &&
      payload.status !== "disconnected"
    )
      return undefined;
    if (typeof payload.timestamp !== "string") return undefined;
    if (
      payload.latencyMs !== undefined &&
      typeof payload.latencyMs !== "number"
    )
      return undefined;
    return message as RegistryEvent;
  }

  return undefined;
}

function isRecord(v: unknown): v is Record<string, any> {
  return typeof v === "object" && v !== null;
}
