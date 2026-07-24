/**
 * Skeleton loading components for terminal UI
 * Provides visual feedback while content is loading
 */

export type SkeletonVariant = "line" | "block" | "card" | "table-row";

export interface SkeletonOptions {
  width: number;
  height?: number;
  variant?: SkeletonVariant;
  animated?: boolean;
  shimmerSpeed?: number;
}

// Animation frame counter for shimmer effect
let animationFrame = 0;

/**
 * Increments the animation frame for shimmer effect
 */
export function advanceAnimationFrame(): void {
  animationFrame = (animationFrame + 1) % 4;
}

/**
 * Gets the current animation frame
 */
export function getAnimationFrame(): number {
  return animationFrame;
}

/**
 * Creates a skeleton loading pattern for terminal display
 */
export function createSkeleton(options: SkeletonOptions): string {
  const { width, height = 1, variant = "line", animated = true } = options;

  switch (variant) {
    case "line":
      return createLineSkeleton(width, height, animated);
    case "block":
      return createBlockSkeleton(width, height, animated);
    case "card":
      return createCardSkeleton(width, animated);
    case "table-row":
      return createTableRowSkeleton(width, animated);
    default:
      return createLineSkeleton(width, height, animated);
  }
}

/**
 * Creates a skeleton with current animation frame
 */
export function createAnimatedSkeleton(options: SkeletonOptions): string {
  return createSkeleton({ ...options, animated: true });
}

/**
 * Creates a simple line skeleton (e.g., for text content)
 */
function createLineSkeleton(
  width: number,
  height: number,
  animated: boolean,
): string {
  const lines: string[] = [];
  for (let i = 0; i < height; i++) {
    const lineWidth = Math.max(10, width - Math.floor(Math.random() * 10));
    const line = animated
      ? createAnimatedLine(lineWidth)
      : createStaticLine(lineWidth);
    lines.push(line);
  }
  return lines.join("\n");
}

/**
 * Creates a block skeleton (e.g., for larger content areas)
 */
function createBlockSkeleton(
  width: number,
  height: number,
  animated: boolean,
): string {
  const lines: string[] = [];
  for (let i = 0; i < height; i++) {
    const line = animated ? createAnimatedLine(width) : createStaticLine(width);
    lines.push(line);
  }
  return lines.join("\n");
}

/**
 * Creates a card skeleton (e.g., for deployment or contract cards)
 */
function createCardSkeleton(width: number, animated: boolean): string {
  const lines: string[] = [];
  const contentWidth = Math.max(20, width - 4);

  // Title line
  lines.push(createLineSkeleton(contentWidth, 1, animated));
  // Spacer
  lines.push("");
  // Content lines
  lines.push(createLineSkeleton(contentWidth, 2, animated));

  return lines.join("\n");
}

/**
 * Creates a table row skeleton (e.g., for list items)
 */
function createTableRowSkeleton(width: number, animated: boolean): string {
  const colWidths = [3, 10, 8, 40]; // age, network, category/count, contractId
  const totalWidth =
    colWidths.reduce((sum, w) => sum + w, 0) + (colWidths.length - 1) * 2;
  const adjustedWidth = Math.min(width, totalWidth);

  const cols: string[] = [];
  let remainingWidth = adjustedWidth;

  for (let i = 0; i < colWidths.length && remainingWidth > 0; i++) {
    const colWidth = Math.min(colWidths[i], remainingWidth);
    const col = animated
      ? createAnimatedLine(colWidth)
      : createStaticLine(colWidth);
    cols.push(col);
    remainingWidth -= colWidth + 2; // 2 for spacing
  }

  return cols.join("  ");
}

/**
 * Creates an animated line with shimmer effect
 */
function createAnimatedLine(width: number): string {
  const chars = ["░", "▒", "▓", "█"];
  let line = "";
  const frame = getAnimationFrame();
  for (let i = 0; i < width; i++) {
    const charIndex = (i + frame) % chars.length;
    line += chars[charIndex];
  }
  return line;
}

/**
 * Creates a static line (no animation)
 */
function createStaticLine(width: number): string {
  return "░".repeat(width);
}

/**
 * Creates skeleton items for a list
 */
export function createSkeletonList(
  count: number,
  width: number,
  variant: SkeletonVariant = "table-row",
): string[] {
  const items: string[] = [];
  for (let i = 0; i < count; i++) {
    items.push(createSkeleton({ width, variant }));
  }
  return items;
}

/**
 * Creates a loading message with skeleton
 */
export function createLoadingMessage(message: string, width: number): string {
  const padding = Math.max(0, Math.floor((width - message.length) / 2));
  const paddedMessage = " ".repeat(padding) + message;
  return paddedMessage;
}

/**
 * Creates an accessible loading indicator for screen readers
 */
export function createAccessibleLoadingIndicator(
  contentType: string,
  width: number,
): string {
  const message = `Loading ${contentType}...`;
  const padding = Math.max(0, Math.floor((width - message.length) / 2));
  return " ".repeat(padding) + message;
}

/**
 * Creates a skeleton with accessibility label
 */
export function createAccessibleSkeleton(
  options: SkeletonOptions & { contentType?: string },
): string {
  const { contentType, ...skeletonOptions } = options;
  const skeleton = createSkeleton(skeletonOptions);

  if (contentType) {
    const indicator = createAccessibleLoadingIndicator(
      contentType,
      skeletonOptions.width,
    );
    return `${indicator}\n${skeleton}`;
  }

  return skeleton;
}

/**
 * Creates a shimmer animation frame
 */
export function createShimmerFrame(width: number, frame: number): string {
  const chars = ["░", "▒", "▓", "█"];
  let line = "";
  for (let i = 0; i < width; i++) {
    const charIndex = (i + frame) % chars.length;
    line += chars[charIndex];
  }
  return line;
}
