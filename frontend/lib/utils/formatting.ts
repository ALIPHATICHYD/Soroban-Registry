/**
 * Format an address or hash by showing start and end characters with separator
 * @param address The address or hash to format
 * @param startChars Number of characters to show at start (default: 6)
 * @param endChars Number of characters to show at end (default: 4)
 * @param separator Separator between start and end (default: '...')
 * @returns Formatted address string
 */
export function formatAddress(
  address: string,
  startChars = 6,
  endChars = 4,
  separator = "...",
): string {
  if (!address || address.length <= startChars + endChars) {
    return address;
  }
  return `${address.slice(0, startChars)}${separator}${address.slice(-endChars)}`;
}

/**
 * Format a contract ID with specific character counts
 * @param contractId The contract ID to format
 * @returns Formatted contract ID (8 chars start, 6 chars end)
 */
export function formatContractId(contractId: string): string {
  return formatAddress(contractId, 8, 6);
}

/**
 * Format a public key or account address
 * @param address The address to format
 * @returns Formatted address (6 chars start, 4 chars end)
 */
export function formatPublicKey(address: string): string {
  return formatAddress(address, 6, 4);
}

/**
 * Format a transaction hash
 * @param hash The transaction hash to format
 * @returns Formatted hash (8 chars start)
 */
export function formatTransactionHash(hash: string): string {
  return formatAddress(hash, 8, 0, "…");
}

/**
 * Format a string to show end characters only
 * @param text The text to format
 * @param endChars Number of characters to show at end (default: 13)
 * @param separator Separator (default: '…')
 * @returns Formatted text
 */
export function formatShortenedText(
  text: string,
  maxLength = 14,
  suffix = "…",
): string {
  if (text.length > maxLength) {
    return text.slice(0, maxLength - 1) + suffix;
  }
  return text;
}
