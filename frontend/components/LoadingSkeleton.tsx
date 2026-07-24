interface LoadingSkeletonProps {
  width?: string;
  height?: string;
  className?: string;
  variant?: "rectangular" | "circular" | "text";
}

export default function LoadingSkeleton({
  width = "100%",
  height = "1rem",
  className = "",
  variant = "rectangular",
}: LoadingSkeletonProps) {
  const baseClasses = "animate-pulse bg-muted";

  const variantClasses = {
    rectangular: "rounded-md",
    circular: "rounded-full",
    text: "rounded",
  };

  return (
    <div
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
      style={{ width, height }}
      role="status"
      aria-label="Loading content"
      aria-live="polite"
    >
      <span className="sr-only">Loading...</span>
    </div>
  );
}
