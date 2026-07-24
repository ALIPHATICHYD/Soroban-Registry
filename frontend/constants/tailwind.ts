/**
 * Reusable Tailwind CSS class constants
 * Consolidates repeated className patterns for consistency and maintainability
 */

// Network-related colors
export const NETWORK_COLORS = {
  mainnet: "bg-green-500/10 text-green-600 border-green-500/20",
  testnet: "bg-blue-500/10 text-blue-600 border-blue-500/20",
  futurenet: "bg-purple-500/10 text-purple-600 border-purple-500/20",
} as const;

export const NETWORK_DOTS = {
  mainnet: "bg-green-500",
  testnet: "bg-blue-500",
  futurenet: "bg-purple-500",
} as const;

// Status badge colors
export const STATUS_COLORS = {
  success: "bg-green-50/40 dark:bg-green-900/10",
  error: "bg-red-50/40 dark:bg-red-900/10",
  warning: "bg-yellow-50/40 dark:bg-yellow-900/10",
  info: "bg-blue-50/40 dark:bg-blue-900/10",
} as const;

// Text styling patterns
export const TEXT_MUTED =
  "text-muted-foreground hover:text-foreground hover:bg-accent";

export const TEXT_TRUNCATE = "truncate";

export const TEXT_SUBTLE = "text-sm text-muted-foreground";

// Form control styling
export const FORM_CONTROL_BASE =
  "w-full px-3 py-2 rounded-lg border border-border bg-background";

export const FORM_INPUT = `${FORM_CONTROL_BASE} text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all`;

export const FORM_TEXTAREA = `${FORM_CONTROL_BASE} text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all resize-none`;

export const FORM_SELECT = `${FORM_CONTROL_BASE} text-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all cursor-pointer`;

// Card styling
export const CARD_BASE =
  "rounded-lg border border-border bg-card text-card-foreground";

export const CARD_HOVER =
  "hover:border-primary/50 hover:shadow-md transition-all";

// Button styling
export const BUTTON_BASE =
  "inline-flex items-center justify-center rounded-lg font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2";

export const BUTTON_PRIMARY = `${BUTTON_BASE} bg-primary text-primary-foreground hover:bg-primary/90`;

export const BUTTON_SECONDARY = `${BUTTON_BASE} bg-secondary text-secondary-foreground hover:bg-secondary/80`;

// Badge styling
export const BADGE_BASE =
  "inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium";

export const BADGE_DEFAULT = `${BADGE_BASE} bg-secondary text-secondary-foreground`;

// Spinner/Loading
export const SPINNER_ANIMATION = "animate-spin";

// Tooltip styling
export const TOOLTIP_BASE =
  "absolute z-50 px-2 py-1 rounded-md bg-popover text-popover-foreground text-sm shadow-md pointer-events-none";
