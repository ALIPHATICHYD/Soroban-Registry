'use client';


export interface BadgeConfig {
  label: string;
  className: string;
  Icon?: React.ComponentType<{ className?: string }>;
}

export interface BadgeProps {
  status: string;
  config: Record<string, BadgeConfig>;
  defaultConfig?: BadgeConfig;
  size?: 'sm' | 'md';
}

/**
 * Generic Badge component for displaying status with icon and label
 * Provides configurable styling, icons, and labels based on status
 *
 * @example
 * ```tsx
 * const verificationConfig = {
 *   approved: { label: 'Verified', className: '...', Icon: CheckCircle2 },
 *   rejected: { label: 'Rejected', className: '...', Icon: ShieldX },
 * };
 *
 * <Badge status="approved" config={verificationConfig} size="md" />
 * ```
 */
export default function Badge({
  status,
  config,
  defaultConfig,
  size = 'sm',
}: BadgeProps) {
  const badgeConfig = config[status] || defaultConfig;

  if (!badgeConfig) {
    console.warn(`Badge: No configuration found for status "${status}" and no default provided`);
    return null;
  }

  const { label, className, Icon } = badgeConfig;
  const iconSize = size === 'md' ? 'w-4 h-4' : 'w-3 h-3';
  const textSize = size === 'md' ? 'text-xs' : 'text-[10px]';
  const padding = size === 'md' ? 'px-2.5 py-1' : 'px-2 py-0.5';

  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full border ${padding} ${textSize} font-semibold uppercase tracking-wide ${className}`}
    >
      {Icon && <Icon className={iconSize} />}
      {label}
    </span>
  );
}
