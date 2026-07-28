import { Tag } from "../types/tag";
import { MOCK_TAGS } from "../mocks/tags.mock";
import { globalCache } from "../utils/cache";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "";
const USE_MOCKS = process.env.NEXT_PUBLIC_USE_MOCKS === "true";
const CACHE_TTL_SECONDS = 300; // 5 minutes

export async function getTags(prefix: string): Promise<Tag[]> {
  const normalizedPrefix = prefix.trim().toLowerCase();

  if (!normalizedPrefix) {
    return [];
  }

  const cacheKey = `tags:${normalizedPrefix}`;
  const cachedResult = globalCache.get<Tag[]>(cacheKey);

  if (cachedResult) {
    return cachedResult;
  }

  let result: Tag[];

  if (!USE_MOCKS) {
    // Real API call
    const res = await fetch(
      `${API_URL}/api/tags?prefix=${encodeURIComponent(normalizedPrefix)}`,
    );
    if (!res.ok) {
      throw new Error(`Failed to fetch tags: ${res.status}`);
    }
    result = await res.json();
  } else {
    // Mock fallback
    const filteredTags = MOCK_TAGS.filter((tag) =>
      tag.name.toLowerCase().includes(normalizedPrefix),
    );

    const uniqueTagsMap = new Map<string, Tag>();
    const uniqueNamesSet = new Set<string>();

    filteredTags.forEach((tag) => {
      if (
        !uniqueTagsMap.has(tag.id) &&
        !uniqueNamesSet.has(tag.name.toLowerCase())
      ) {
        uniqueTagsMap.set(tag.id, tag);
        uniqueNamesSet.add(tag.name.toLowerCase());
      }
    });

    const uniqueTags = Array.from(uniqueTagsMap.values());
    uniqueTags.sort((a, b) => b.usageCount - a.usageCount);
    result = uniqueTags.slice(0, 5);
  }

  globalCache.set(cacheKey, result, CACHE_TTL_SECONDS);
  return result;
}
