export interface ContractMetadata {
  contractId: string;
  name: string;
  network: string;
  wasmHash: string;
  version: string;
  sourceHash?: string;
  auditScore?: number;
  tags?: string[];
}

export interface PinnedContractMetadata {
  cid: string;
  bytes: number;
  pinnedAt: string;
  metadata: ContractMetadata;
  gateways: string[];
}

function canonicalize(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(",")}]`;

  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canonicalize(record[key])}`)
    .join(",")}}`;
}

function fnv1a(input: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < input.length; index += 1) {
    hash ^= input.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function cidForPayload(payload: string): string {
  const chunks = [
    fnv1a(payload),
    fnv1a(`soroban:${payload.length}:${payload}`),
    fnv1a(`${payload}:registry`),
    fnv1a(`${payload}:ipfs`),
  ].join("");

  return `bafy${BigInt(`0x${chunks}`).toString(32).padStart(52, "0").slice(0, 52)}`;
}

export function createGatewayUrls(
  cid: string,
  gateways = ["https://ipfs.io/ipfs", "https://cloudflare-ipfs.com/ipfs"],
) {
  return gateways.map((gateway) => `${gateway.replace(/\/$/, "")}/${cid}`);
}

export function pinContractMetadata(
  metadata: ContractMetadata,
  pinnedAt = new Date().toISOString(),
): PinnedContractMetadata {
  const payload = canonicalize(metadata);
  const cid = cidForPayload(payload);

  return {
    cid,
    bytes: new TextEncoder().encode(payload).length,
    pinnedAt,
    metadata,
    gateways: createGatewayUrls(cid),
  };
}

export function verifyPinnedContract(pin: PinnedContractMetadata): boolean {
  return pin.cid === pinContractMetadata(pin.metadata, pin.pinnedAt).cid;
}

export function retrievePinnedContract(
  cid: string,
  pins: PinnedContractMetadata[],
) {
  return pins.find((pin) => pin.cid === cid) ?? null;
}
