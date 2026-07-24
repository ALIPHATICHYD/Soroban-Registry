export function parseContractIdsFromSearch(search: string): string[] {
  const params = new URLSearchParams(
    search.startsWith("?") ? search.slice(1) : search,
  );
  const raw = params.get("contracts");
  if (!raw) return [];
  return raw
    .split(",")
    .map((id) => id.trim())
    .filter(Boolean);
}

export function encodeContractIdsToSearch(contractIds: string[]): string {
  const ids = contractIds.map((id) => id.trim()).filter(Boolean);
  const params = new URLSearchParams();
  if (ids.length > 0) params.set("contracts", ids.join(","));
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

export function buildCompareUrl(
  contractIds: string[],
  basePath = "/compare",
): string {
  return `${basePath}${encodeContractIdsToSearch(contractIds)}`;
}

export function replaceUrlContractIds(
  contractIds: string[],
  basePath = "/compare",
) {
  if (typeof window === "undefined") return;
  const nextUrl = buildCompareUrl(contractIds, basePath);
  window.history.replaceState(null, "", nextUrl);
}
