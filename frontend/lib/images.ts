import { decode } from "blurhash";

/**
 * Image utility functions and constants for optimized image rendering
 */

/**
 * Responsive image size breakpoints
 * Used with the 'sizes' attribute in Next.js Image
 */
export const IMAGE_SIZES = {
  /** Full width - for hero sections */
  FULL: "100vw",
  /** Half width - for side-by-side layouts */
  HALF: "(max-width: 768px) 100vw, 50vw",
  /** 3-column grids Third width - for */
  THIRD: "(max-width: 640px) 100vw, (max-width: 768px) 50vw, 33vw",
  /** Quarter width - for 4-column grids */
  QUARTER: "(max-width: 640px) 50vw, (max-width: 768px) 33vw, 25vw",
  /** Thumbnail size */
  THUMBNAIL: "(max-width: 640px) 50vw, (max-width: 1024px) 33vw, 20vw",
  /** Avatar size */
  AVATAR: "40px",
  /** Small icon */
  ICON_SM: "24px",
  /** Medium icon */
  ICON_MD: "32px",
  /** Large icon */
  ICON_LG: "48px",
} as const;

/**
 * Image quality presets for optimization
 */
export const IMAGE_QUALITY = {
  /** Low quality - for thumbnails, placeholders */
  LOW: 40,
  /** Medium quality - for regular images */
  MEDIUM: 60,
  /** High quality - for hero images, detailed graphics */
  HIGH: 80,
  /** Maximum quality - for full-resolution images */
  MAX: 95,
} as const;

/**
 * Default image configurations for different use cases
 */
export const IMAGE_CONFIG = {
  /** Card image - medium size with good quality */
  CARD: {
    sizes: IMAGE_SIZES.THIRD,
    quality: IMAGE_QUALITY.MEDIUM,
    priority: false,
  },
  /** Hero image - full width with high quality */
  HERO: {
    sizes: IMAGE_SIZES.FULL,
    quality: IMAGE_QUALITY.HIGH,
    priority: true,
  },
  /** Thumbnail - small with lower quality for performance */
  THUMBNAIL: {
    sizes: IMAGE_SIZES.THUMBNAIL,
    quality: IMAGE_QUALITY.LOW,
    priority: false,
  },
  /** Avatar - fixed small size */
  AVATAR: {
    sizes: IMAGE_SIZES.AVATAR,
    quality: IMAGE_QUALITY.MEDIUM,
    priority: true,
  },
} as const;

/**
 * Validates if a URL is a valid image URL
 */
export function isValidImageUrl(url: string | undefined | null): boolean {
  if (!url) return false;

  try {
    const parsed = new URL(url);
    // Check for common image extensions or data URLs
    const imageExtensions = [
      ".jpg",
      ".jpeg",
      ".png",
      ".gif",
      ".webp",
      ".avif",
      ".svg",
      ".ico",
    ];
    const hasExtension = imageExtensions.some((ext) =>
      parsed.pathname.toLowerCase().endsWith(ext),
    );
    const isDataUrl = url.startsWith("data:image/");
    const isBlob = url.startsWith("blob:");

    return hasExtension || isDataUrl || isBlob;
  } catch {
    return false;
  }
}

/**
 * Gets the appropriate sizes string based on grid columns
 */
export function getGridSizes(columnCount: number): string {
  const breakpoints = [
    { maxWidth: 640, size: "100vw" },
    { maxWidth: 768, size: "100vw" },
    { maxWidth: 1024, size: "50vw" },
    { maxWidth: 1280, size: `${100 / Math.min(columnCount, 4)}vw` },
  ];

  return breakpoints
    .map((b) => `(max-width: ${b.maxWidth}px) ${b.size}`)
    .join(", ");
}

/**
 * Generates a low-quality image placeholder (LQIP) data URL
 * SVG color fallback used when blurhash is unavailable
 */
export function generateSolidPlaceholder(color: string): string {
  return `data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'%3E%3Crect fill='${encodeURIComponent(color)}'/%3E%3C/svg%3E`;
}

export interface BlurHashDecodeOptions {
  width?: number;
  height?: number;
  punch?: number;
  fallbackColor?: string;
}

function canUseCanvas(): boolean {
  return typeof document !== "undefined";
}

function createCanvas(width: number, height: number): HTMLCanvasElement | null {
  if (!canUseCanvas()) return null;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  return canvas;
}

function toPngDataUrl(
  pixels: Uint8ClampedArray,
  width: number,
  height: number,
): string | null {
  const canvas = createCanvas(width, height);
  if (!canvas) return null;

  const ctx = canvas.getContext("2d");
  if (!ctx) return null;

  const imageData = ctx.createImageData(width, height);
  imageData.data.set(pixels);
  ctx.putImageData(imageData, 0, 0);
  return canvas.toDataURL("image/png");
}

/**
 * Decodes a blurhash into a PNG data URL for `next/image` blur placeholders.
 * Falls back to a solid-color SVG if decoding/canvas is unavailable.
 */
export function generateBlurHashPlaceholder(
  blurHash: string | null | undefined,
  options: BlurHashDecodeOptions = {},
): string {
  const {
    width = 32,
    height = 32,
    punch = 1,
    fallbackColor = "#e5e7eb",
  } = options;

  if (!blurHash) {
    return generateSolidPlaceholder(fallbackColor);
  }

  try {
    const pixels = decode(blurHash, width, height, punch);
    const dataUrl = toPngDataUrl(pixels, width, height);
    return dataUrl ?? generateSolidPlaceholder(fallbackColor);
  } catch {
    return generateSolidPlaceholder(fallbackColor);
  }
}

/**
 * Supported image MIME types in order of preference (best first)
 */
export const SUPPORTED_FORMATS = [
  "image/avif",
  "image/webp",
  "image/jpeg",
  "image/png",
  "image/gif",
  "image/svg+xml",
] as const;

/**
 * File size limits for different image types (in bytes)
 */
export const IMAGE_SIZE_LIMITS = {
  AVATAR: 500 * 1024, // 500KB
  THUMBNAIL: 1024 * 1024, // 1MB
  HERO: 2 * 1024 * 1024, // 2MB
  GENERAL: 5 * 1024 * 1024, // 5MB
} as const;
