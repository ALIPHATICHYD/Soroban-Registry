export function formatSince(ts: number, nowTs: number): string {
  const deltaSec = Math.max(0, Math.floor((nowTs - ts) / 1000));
  if (deltaSec < 60) return `${deltaSec}s`;
  const min = Math.floor(deltaSec / 60);
  if (min < 60) return `${min}m`;
  const hr = Math.floor(min / 60);
  return `${hr}h`;
}

export function clampStr(s: string, maxLen: number): string {
  if (s.length <= maxLen) return s;
  if (maxLen <= 1) return s.slice(0, maxLen);
  return `${s.slice(0, maxLen - 1)}…`;
}
